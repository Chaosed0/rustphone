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
use bsp_query::*;

mod player;
use player::Player;

use std::{f32::consts::PI, ffi::c_void, sync::atomic::{AtomicBool, Ordering} };
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
	let mesh_vs = exe_dir_path.join("assets/shaders/mesh.vs").to_str().expect("No vpath").to_owned();
	let mesh_fs = exe_dir_path.join("assets/shaders/mesh.fs").to_str().expect("No fpath").to_owned();
	let mut mesh_shader = rl.load_shader(&thread, Some(mesh_vs.as_str()), Some(mesh_fs.as_str()));

	assert!(default_shader.is_shader_valid() && cutout_shader.is_shader_valid() && skybox_shader.is_shader_valid() && mesh_shader.is_shader_valid(), "Error compiling shaders");

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

	if let Some(lightgrid) = &bsp.lightgrid {
		bsp_render.build_lightgrid_data(lightgrid);

		let dist_loc = mesh_shader.get_shader_location("lgData.dist");
		let size_x_loc = mesh_shader.get_shader_location("lgData.size_x");
		let size_y_loc = mesh_shader.get_shader_location("lgData.size_y");
		let size_z_loc = mesh_shader.get_shader_location("lgData.size_z");
		let mins_loc = mesh_shader.get_shader_location("lgData.mins");
		mesh_shader.set_shader_value(dist_loc, to_wld(lightgrid.header.grid_dist));
		mesh_shader.set_shader_value(size_x_loc, lightgrid.header.grid_size[1]);
		mesh_shader.set_shader_value(size_y_loc, lightgrid.header.grid_size[2]);
		mesh_shader.set_shader_value(size_z_loc, lightgrid.header.grid_size[0]);
		mesh_shader.set_shader_value(mins_loc, to_wld(lightgrid.header.grid_mins));
	}

	//let mut time = 0f32;

	//println!("ENTITIES: {:?}", bsp.entities);

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

	let mut cube_pos = Vector3::Y * 64f32;
	let mut cube_dir = 1f32;

    while !rl.window_should_close() {
		let dt = rl.get_frame_time();

		cube_pos += cube_dir * Vector3::X * 64f32 * dt;
		if cube_dir > 0f32 && cube_pos.x > 256f32 {
			cube_dir = -1f32;
		} else if cube_dir < 0f32 && cube_pos.x < 0f32 {
			cube_dir = 1f32;
		}

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
            let rend = ray_intersect(&bsp_clipq, raycast_position.unwrap(), raycast_dir, f32::INFINITY, *DPASS);
            raycast_end = Some(match rend {
				Some(rend) => rend.position,
				None => raycast_position.unwrap() + raycast_dir * 9999f32
			});

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

        d.clear_background(Color::GRAY);
		unsafe { raylib::ffi::rlEnableDepthTest() };
		//unsafe { raylib::ffi::rlDisableBackfaceCulling() };
		unsafe { raylib::ffi::rlSetClipPlanes(1f64, 100000f64) };

		d.draw_mode3D(cam, |mut d3d|
		{
			let modelview: Matrix = unsafe { raylib::ffi::rlGetMatrixModelview().try_into().unwrap() };
			let projection: Matrix = unsafe { raylib::ffi::rlGetMatrixProjection().try_into().unwrap() };
			bsp_render.render(&textures, &lightmaps, &bsp, modelview * projection, cam);

			d3d.draw_shader_mode(&mut mesh_shader, |mut dsm| {
				bsp_render.bind_lightgrid_data();
				//dsm.draw_cube(cube_pos, 64f32, 64f32, 64f32, Color::WHITE);
				dsm.draw_cube(cam.position + cam.forward() * 16f32, 4f32, 4f32, 4f32, Color::WHITE);
			});

            if let Some(end) = raycast_end &&
                let Some(pos) = raycast_position {
                d3d.draw_sphere_wires(end, 4f32, 8, 8, Color::ORANGE); 
                d3d.draw_ray(Ray::new(pos, end - pos), Color::RED);
            }

			//render_lightgrid_leafs(&mut d3d, &bsp);
		});

		d.draw_texture_ex(&lightmaps[0], Vector2::new(10f32, 10f32), 0f32, 0.2f32, Color::WHITE);

		d.draw_fps(10, 10);
    }

	return Ok(());
}

fn render_lightgrid_leafs(d3d: &mut RaylibMode3D<'_, RaylibDrawHandle>, bsp: &Bsp) {
	let Some(lightgrid) = bsp.lightgrid.as_ref() else { return; };
	render_lightgrid_leafs_recursive(d3d, &lightgrid, &lightgrid.nodes[lightgrid.header.root_node as usize], Vector3::ZERO, &lightgrid.header);
}

fn render_lightgrid_leafs_recursive(d3d: &mut RaylibMode3D<'_, RaylibDrawHandle>, lightgrid: &Lightgrid, node: &LightgridNode, pos: Vector3, header: &LightgridHeader) {
	for i in 0..node.children.len() {
		render_lightgrid_node(d3d, lightgrid, node.children[i], pos + get_octant_offset(i as u32, node.division_point), header);
	}
}

fn get_octant_offset(i: u32, div_point: [i32;3]) -> Vector3 {
	return Vector3::new(
		if (i & 4) > 0 { 0f32 } else { div_point[0] as f32 },
		if (i & 2) > 0 { 0f32 } else { div_point[1] as f32 },
		if (i & 1) > 0 { 0f32 } else { div_point[2] as f32 }
	);
}

fn render_lightgrid_node(d3d: &mut RaylibMode3D<'_, RaylibDrawHandle>, lightgrid: &Lightgrid, node_index: u32, pos: Vector3, header: &LightgridHeader) {
	if (node_index & LIGHTGRID_OCCLUDED) > 0 {
	} else if (node_index & LIGHTGRID_LEAF) > 0 {
		let leaf_index = node_index & (!LIGHTGRID_LEAF);
		let leaf = &lightgrid.leafs[leaf_index as usize];
		render_lightgrid_leaf(d3d, lightgrid, leaf, pos, header);
	} else {
		render_lightgrid_leafs_recursive(d3d, lightgrid, &lightgrid.nodes[node_index as usize], pos, header);
	}
}

fn render_lightgrid_leaf(d3d: &mut RaylibMode3D<'_, RaylibDrawHandle>, lightgrid: &Lightgrid, leaf: &LightgridLeaf, pos: Vector3, header: &LightgridHeader) {
	let leaf_pos = Vector3::new(leaf.mins[0] as f32, leaf.mins[1] as f32, leaf.mins[2] as f32);

	for z in 0..leaf.size[2] {
		for y in 0..leaf.size[1] {
			for x in 0..leaf.size[0] {
				let sample_pos = Vector3::new(x as f32, y as f32, z as f32);
				let sample_idx = leaf.sample_start_idx + (x + y * leaf.size[0] + z * (leaf.size[0] * leaf.size[1])) as usize;
				let sample_set = lightgrid.samples[sample_idx as usize];

				if sample_set.used_styles > 0 && sample_set.used_styles != 0xff {
					let color_u8 = lightgrid.samples[sample_idx as usize].samples[0].color;
					let color = Color::new(color_u8[0], color_u8[1], color_u8[2], 255u8);
					let position = header.grid_mins + (leaf_pos + sample_pos) * header.grid_dist;

					d3d.draw_sphere(to_wld(position), 1f32, color);
				}
			}
		}
	}
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
	player.pitch = player.pitch.clamp(-PI * 0.49f32, PI * 0.49f32);

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
