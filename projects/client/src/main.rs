mod transport;
use transport::Transport;

mod bsp;
use bsp::*;

mod bsp_entity;

mod bsp_render;
use bsp_render::*;

mod lit;
use lit::*;

mod palette;
use palette::PALETTE;

mod bsp_query;

mod player;
use player::Player;

use std::ffi::c_void;
use raylib::prelude::*;
use gns::GnsGlobal;
use std::net::Ipv4Addr;
use shared::Message;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>
{
    // Initial the global networking state. Note that this instance must be unique per-process.
    let gns_global = GnsGlobal::get().expect("no global networking state");

    let (mut rl, thread) = raylib::init()
        .size(1920, 1080)
        .title("Hello, World")
        .build();

    let transport = Transport::new(gns_global.clone(), Ipv4Addr::LOCALHOST.into(), 27821).expect("connection failed");
	//let bsp = load_bsp("assets/box.bsp");
	let bsp = load_bsp("assets/qbj3_chaosed0.bsp");
	let mut bsp_render = BspRender::new();

	bsp_render.load_skybox("assets/skybox/mak_cloudysky5");

	let exe_dir_path = std::env::current_exe().expect("no exe path").parent().expect("PARENT").to_owned();
	let default_vs = exe_dir_path.join("assets/shaders/default.vs").to_str().expect("No vpath").to_owned();
	let default_fs = exe_dir_path.join("assets/shaders/default.fs").to_str().expect("No fpath").to_owned();
	let default_shader = rl.load_shader(&thread, Some(default_vs.as_str()), Some(default_fs.as_str()));
	let cutout_fs = exe_dir_path.join("assets/shaders/cutout.fs").to_str().expect("No fpath").to_owned();
	let cutout_shader = rl.load_shader(&thread, Some(default_vs.as_str()), Some(cutout_fs.as_str()));
	let skybox_vs = exe_dir_path.join("assets/shaders/skybox.vs").to_str().expect("No vpath").to_owned();
	let skybox_fs = exe_dir_path.join("assets/shaders/skybox.fs").to_str().expect("No fpath").to_owned();
	let skybox_shader = rl.load_shader(&thread, Some(skybox_vs.as_str()), Some(skybox_fs.as_str()));

	assert!(default_shader.is_shader_valid() && cutout_shader.is_shader_valid() && skybox_shader.is_shader_valid(), "Error compiling shaders");

	bsp_render.load_shaders(&default_shader, &cutout_shader, &skybox_shader);

	let textures = {
		let mut image_gen_set = tokio::task::JoinSet::new();
		for (i, texture) in bsp.textures.iter().enumerate()
		{
			let pixels = texture.pixels.clone();
			let width = texture.width;
			let height = texture.height;

			image_gen_set.spawn(async move { (i, gen_pixels(pixels, width, height)) });
		}

		let mut textures = std::iter::repeat_with(|| Option::<Texture2D>::None).take(bsp.textures.len()).collect::<Vec<_>>();
		while let Some(tup) = image_gen_set.join_next().await
		{
			let (i, pixels) = tup.unwrap();
			let texture = &bsp.textures[i];
			let image = image_from_pixels(pixels, texture.width, texture.height, PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8);
			let tex = rl.load_texture_from_image(&thread, &image)
				.unwrap_or_else(|err| panic!("Could not generate texture from image: {err}"));

			tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_POINT);
			tex.set_texture_wrap(&thread, TextureWrap::TEXTURE_WRAP_REPEAT);
			textures[i] = Some(tex);
		}

		textures.into_iter().map(|t| t.unwrap()).collect::<Vec<Texture2D>>()
	};

	let light_data = pack_lightmaps(&bsp);
	bsp_render.build_buffers(&bsp, &light_data);

	let lightmaps = light_data.lightmaps.into_iter().map(|lm|
		{
			let image = image_from_pixels(lm.bytes, lm.width, lm.height, PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8);
			let tex = rl.load_texture_from_image(&thread, &image)
				.unwrap_or_else(|err| panic!("Could not generate texture from image: {err}"));

			tex.set_texture_filter(&thread, TextureFilter::TEXTURE_FILTER_BILINEAR);
			tex.set_texture_wrap(&thread, TextureWrap::TEXTURE_WRAP_CLAMP);
			return tex;
		}).collect::<Vec<Texture2D>>();

	//let mut time = 0f32;

	println!("ENTITIES: {:?}", bsp.entities);

	rl.set_target_fps(60);

    let origin_str = "origin".to_string();
	let mut cam = Camera3D::perspective(Vector3::ZERO, Vector3::Z, Vector3::Y, 60f32);

    let pos = bsp_entity::of_type(&bsp, "info_player_start").next().unwrap().get_vec3(&origin_str);
    let mut player = Player::new(pos + Vector3::Y);

	rl.disable_cursor();
	rl.set_exit_key(None);

    let mut raycast_position = None;
    let mut raycast_end = None;

    let bsp_visq = bsp_query::BspVisQuery::new(&bsp);
    let bsp_clipq = bsp_query::BspClipQuery::new(&bsp);

    while !rl.window_should_close() {
		let dt = rl.get_frame_time();

		if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
			rl.enable_cursor();
		}

		if rl.is_cursor_hidden() {
			poll_input(&mut rl, &mut player);
		} else if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
			rl.disable_cursor();
		}

		if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
            raycast_position = Some(cam.position);
            let raycast_dir = cam.target - cam.position;
            let rend = bsp_query::ray_intersect(&bsp_clipq, raycast_position.unwrap(), raycast_dir, f32::INFINITY);
            raycast_end = Some(rend.unwrap_or(raycast_position.unwrap() + raycast_dir * 9999f32));
            //println!("{:?}->{:?}", raycast_position, raycast_end);
		}

        //let leaf = bsp_query::get_leaf_containing_point(&bsp, cam.position);
        //println!("LEAF {:?}: {:?} {:?} {:?} {:?}", cam.position, leaf.contents, leaf.firstmarksurface, leaf.nummarksurfaces, leaf.visofs);

        transport.poll_messages(message_handler);
        gns_global.poll_callbacks();

        player.update(&bsp_clipq, dt);
        cam.position = player.pos + Vector3::Y * 16f32;
        cam.target = cam.position + player.forward();

        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::BLACK);
		unsafe { raylib::ffi::rlEnableDepthTest() };
		//unsafe { raylib::ffi::rlDisableBackfaceCulling() };
		unsafe { raylib::ffi::rlSetClipPlanes(1f64, 100000f64) };

		d.draw_mode3D(cam, |mut d3d|
		{
			let modelview: Matrix = unsafe { raylib::ffi::rlGetMatrixModelview().try_into().unwrap() };
			let projection: Matrix = unsafe { raylib::ffi::rlGetMatrixProjection().try_into().unwrap() };
			bsp_render.render(&textures, &lightmaps, &bsp, modelview * projection, cam);

            if let Some(end) = raycast_end &&
                let Some(pos) = raycast_position {
                d3d.draw_sphere_wires(end, 4f32, 8, 8, Color::ORANGE); 
                d3d.draw_ray(Ray::new(pos, end - pos), Color::RED);
            }
		});

		d.draw_texture_ex(&lightmaps[0], Vector2::new(10f32, 10f32), 0f32, 0.2f32, Color::WHITE);

		d.draw_fps(10, 10);
    }

	return Ok(());
}

fn gen_pixels(pixels: Vec<u8>, width: u32, height: u32) -> Vec<u8>
{
	if width == 0 || height == 0 {
		return [255, 0, 255, 255, 255, 0, 255, 255, 255, 0, 255, 255, 255, 0, 255, 255].into();
	}

	let pixels_u8 = pixels.iter().flat_map(|b| {
		let col = PALETTE[*b as usize].to_le_bytes();
		return [col[2], col[1], col[0], 255];
	}).collect::<Vec<u8>>();

	return pixels_u8;
}

fn image_from_pixels(pixels: Vec<u8>, width: u32, height: u32, format: PixelFormat) -> Image
{
	unsafe {
		let mut pixels = std::mem::ManuallyDrop::new(pixels);

		return Image::from_raw(raylib::ffi::Image {
			data: pixels.as_mut_ptr() as *mut c_void,
			width: if width == 0 { 2 } else { width as i32 },
			height: if height == 0 { 2 } else { height as i32 },
			format: format as i32,
			mipmaps: 1
		});
	}
}

fn poll_input(rl: &mut RaylibHandle, player : &mut Player)
{
	const ROT_SPEED: f32 = 0.5f32;

    let mouse_delta = rl.get_mouse_delta();
	let dt = rl.get_frame_time();
    let rot_speed = ROT_SPEED * dt;

	player.yaw -= mouse_delta.x * rot_speed;
	player.pitch += mouse_delta.y * rot_speed;
	player.pitch = player.pitch.clamp(-89f32, 89f32);

    let mut movement = Vector3::ZERO;

	if rl.is_key_down(KeyboardKey::KEY_W) { movement.z += 1f32; }
	if rl.is_key_down(KeyboardKey::KEY_A) { movement.x += 1f32; }
	if rl.is_key_down(KeyboardKey::KEY_S) { movement.z -= 1f32; }
	if rl.is_key_down(KeyboardKey::KEY_D) { movement.x -= 1f32; }
	if rl.is_key_down(KeyboardKey::KEY_SPACE) { movement.y += 1f32; }
	if rl.is_key_down(KeyboardKey::KEY_Q) { movement.y += 1f32; }
	if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) { movement.y -= 1f32; }
	if rl.is_key_down(KeyboardKey::KEY_E) { movement.y -= 1f32; }

    player.movement = movement;

    if rl.is_key_pressed(KeyboardKey::KEY_SPACE) { player.jump = true; }
    player.sprint = rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT);

    if rl.is_key_pressed(KeyboardKey::KEY_ZERO) { player.free_move = !player.free_move; }
}

fn message_handler(_msg: Message) {
}

fn print_bsp_tree(bsp: &Bsp, idx: i32, ind: usize) {
    let spc = " ".repeat(ind);

    if idx < 0 {
        println!("{spc} {idx}: LEAF {:?}", bsp.leafs[-(idx + 1) as usize].contents);
    } else {
        let node = &bsp.nodes[idx as usize];
        let plane = &bsp.planes[node.plane_index as usize];
        println!("{spc} {idx}: NODE {:?} {:?}", plane.normal, plane.dist);
        print_bsp_tree(bsp, node.children[0], ind + 1);
        print_bsp_tree(bsp, node.children[1], ind + 1);
    }
}
