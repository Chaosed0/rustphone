mod transport;
use transport::Transport;

mod bsp;
use bsp::*;

mod bsp_render;
use bsp_render::*;

mod lit;
use lit::*;

mod palette;
use palette::PALETTE;

mod bsp_query;

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
	let bsp = load_bsp("assets/qbj3_chaosed0.bsp");
	//let bsp = load_bsp("assets/box.bsp");
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

	//println!("ENTITIES: {:?}", bsp.entities);

	rl.set_target_fps(60);

	let mut cam = Camera3D::perspective(Vector3::ZERO, Vector3::Z, Vector3::Y, 60f32);

	rl.disable_cursor();
	rl.set_exit_key(None);

    //let leaf = bsp_query::get_leaf_containing_point(&bsp, Vector3::new(0f32, 0f32, 2f32));

    //println!("LEAF0: {:?} {:?} {:?} {:?}", leaf.contents, leaf.firstmarksurface, leaf.nummarksurfaces, leaf.visofs);

    //let leaf = bsp_query::get_leaf_containing_point(&bsp, Vector3::new(0f32, 0f32, -2f32));

    //println!("LEAF1: {:?} {:?} {:?} {:?}", leaf.contents, leaf.firstmarksurface, leaf.nummarksurfaces, leaf.visofs);

    while !rl.window_should_close() {
		//let delta = rl.get_frame_time();

		if rl.is_key_pressed(KeyboardKey::KEY_ESCAPE) {
			rl.enable_cursor();
		}

		if rl.is_cursor_hidden() {
			update_camera(&mut rl, &mut cam);
		}
		else if rl.is_mouse_button_pressed(MouseButton::MOUSE_BUTTON_LEFT) {
			rl.disable_cursor();
		}

        transport.poll_messages(message_handler);
        gns_global.poll_callbacks();

		//time += delta;

        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::BLACK);
		unsafe { raylib::ffi::rlEnableDepthTest() };
		//unsafe { raylib::ffi::rlDisableBackfaceCulling() };
		unsafe { raylib::ffi::rlSetClipPlanes(1f64, 100000f64) };

		d.draw_mode3D(cam, |_|
		{
			let modelview: Matrix = unsafe { raylib::ffi::rlGetMatrixModelview().try_into().unwrap() };
			let projection: Matrix = unsafe { raylib::ffi::rlGetMatrixProjection().try_into().unwrap() };
			bsp_render.render(&textures, &lightmaps, &bsp, modelview * projection, cam);
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

fn update_camera(rl: &mut RaylibHandle, camera : &mut Camera)
{
	const CAMERA_MOVE_SPEED: f32 = 256f32;
	const CAMERA_ROTATION_SPEED: f32 = 0.1f32;

    let mouse_delta = rl.get_mouse_delta();
	let delta = rl.get_frame_time();

    // Camera speeds based on frame time
    let mut speed = CAMERA_MOVE_SPEED * delta;
    let rot_speed = CAMERA_ROTATION_SPEED * delta;

	let mut forward = (camera.target - camera.position).try_normalize().unwrap();
	let up = camera.up;
	let right = forward.cross(up).try_normalize().unwrap();

	forward = forward.rotate_axis(up, -mouse_delta.x * rot_speed);
	forward = forward.rotate_axis(right, -mouse_delta.y * rot_speed);

	if rl.is_key_down(KeyboardKey::KEY_LEFT_SHIFT) { speed *= 3f32; }
	if rl.is_key_down(KeyboardKey::KEY_W) { camera.position += forward * speed; }
	if rl.is_key_down(KeyboardKey::KEY_A) { camera.position -= right * speed; }
	if rl.is_key_down(KeyboardKey::KEY_S) { camera.position -= forward * speed; }
	if rl.is_key_down(KeyboardKey::KEY_D) { camera.position += right * speed; }
	if rl.is_key_down(KeyboardKey::KEY_SPACE) { camera.position += up * speed; }
	if rl.is_key_down(KeyboardKey::KEY_Q) { camera.position += up * speed; }
	if rl.is_key_down(KeyboardKey::KEY_LEFT_CONTROL) { camera.position += up * speed; }
	if rl.is_key_down(KeyboardKey::KEY_E) { camera.position -= up * speed; }

	camera.target = camera.position + forward;
}

fn message_handler(_msg: Message) {
}
