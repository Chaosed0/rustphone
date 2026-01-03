mod transport;
use transport::Transport;

mod bsp;
use bsp::*;

mod bsp_render;
use bsp_render::*;

mod palette;
use palette::PALETTE;

use std::{f32::consts::PI, ffi::c_void};
use raylib::prelude::*;
use gns::GnsGlobal;
use std::net::Ipv4Addr;
use shared::Message;

fn main() {
    // Initial the global networking state. Note that this instance must be unique per-process.
    let gns_global = GnsGlobal::get().expect("no global networking state");

    let (mut rl, thread) = raylib::init()
        .size(640, 480)
        .title("Hello, World")
        .build();

    let transport = Transport::new(gns_global.clone(), Ipv4Addr::LOCALHOST.into(), 27821).expect("connection failed");
	let bsp = load_bsp("assets/qbj3_chaosed0.bsp");
	let mut bsp_render = BspRender::new();
	bsp_render.build_buffers(&bsp);

	let exe_dir_path = std::env::current_exe().expect("no exe path").parent().expect("PARENT").to_owned();
	let vpath = exe_dir_path.join("assets/shaders/vert.glsl").to_str().expect("No vpath").to_owned();
	let fpath = exe_dir_path.join("assets/shaders/frag.glsl").to_str().expect("No fpath").to_owned();
	let shader = rl.load_shader(&thread, Some(vpath.as_str()), Some(fpath.as_str()));

	let textures = bsp.textures.iter()
		.map(|mip_tex| {
			if mip_tex.width == 0 || mip_tex.height == 0 { return generate_missing_texture(); }
			let image = image_from_pixels(mip_tex);
			let tex = rl.load_texture_from_image(&thread, &image)
				.unwrap_or_else(|err| panic!("Could not generate texture from image: {err}"));

			tex.set_texture_wrap(&thread, TextureWrap::TEXTURE_WRAP_REPEAT);
			return tex;
		})
		.collect::<Vec<Texture2D>>();

	let mut time = 0f32;

	/*
	for (i, surf) in bsp.surfs.iter().enumerate()
	{
		println!("SURF {i}: {:?} {:?}", surf.first_edge, surf.num_edges);

		for e in surf.first_edge..(surf.first_edge + surf.num_edges as i32)
		{
			let surf_edge = bsp.surf_edges[e as usize];
			let edge = &bsp.edges[surf_edge.abs() as usize];
			let (v0, v1) = if surf_edge >= 0 { (edge.v0, edge.v1) } else { (edge.v1, edge.v0) };

			println!("   {surf_edge} {v0}->{v1} {:?}->{:?}", bsp.verts[v0 as usize], bsp.verts[v1 as usize]);
		}
	}
	*/

	println!("ENTITIES: {:?}", bsp.entities);

	rl.set_target_fps(60);

	let mut cam = Camera3D::perspective(Vector3::new(0f32, 0f32, 3f32), Vector3::zero(), Vector3::up(), 60f32);
	let mut angle = 0f32;

	rl.disable_cursor();
	rl.set_exit_key(None);

    while !rl.window_should_close() {
		let delta = rl.get_frame_time();

		/*
		const DIST: f32 = 2f32;
		cam.position = Vector3::new(DIST * angle.sin(), 0f32, DIST * angle.cos());
		angle += delta * PI * 0.5;
		*/

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

		time += delta;

        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::WHITE);
		unsafe { raylib::ffi::rlEnableDepthTest() };
		//unsafe { raylib::ffi::rlDisableBackfaceCulling() };
		unsafe { raylib::ffi::rlSetClipPlanes(1f64, 100000f64) };

		d.draw_mode3D(cam, |mut d3d, cam|
		{
			let modelview: Matrix = unsafe { raylib::ffi::rlGetMatrixModelview().try_into().unwrap() };
			let projection: Matrix = unsafe { raylib::ffi::rlGetMatrixProjection().try_into().unwrap() };
			bsp_render.render(&textures, &shader, modelview * projection, time);
		});

		d.draw_fps(10, 10);
    }
}

fn generate_missing_texture() -> Texture2D
{
	let mut colors = [Color::MAGENTA, Color::MAGENTA, Color::MAGENTA, Color::MAGENTA];

	unsafe {
		let image = raylib::ffi::Image {
			data: colors.as_mut_ptr() as *mut c_void,
			width: 2,
			height: 2,
			format: PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 as i32,
			mipmaps: 1
		};

		let tex = raylib::ffi::LoadTextureFromImage(image);
		raylib::ffi::SetTextureWrap(tex, TextureWrap::TEXTURE_WRAP_REPEAT as i32);
		return Texture2D::from_raw(tex);
	}
}

fn image_from_pixels(tex: &MipTex) -> Image
{
	let pixels = tex.pixels.iter().flat_map(|b| {
		let col = PALETTE[*b as usize].to_le_bytes();
		return [col[2], col[1], col[0], 255u8];
	}).collect::<Vec<u8>>();

	unsafe {
		let mut pixels = std::mem::ManuallyDrop::new(pixels);

		return Image::from_raw(raylib::ffi::Image {
			data: pixels.as_mut_ptr() as *mut c_void,
			width: tex.width as i32,
			height: tex.height as i32,
			format: PixelFormat::PIXELFORMAT_UNCOMPRESSED_R8G8B8A8 as i32,
			mipmaps: 1
		});
	}
}

fn update_camera(rl: &mut RaylibHandle, camera : &mut Camera)
{
	const CAMERA_MOVE_SPEED: f32 = 96f32;
	const CAMERA_ROTATION_SPEED: f32 = 0.1f32;

    let mouse_delta = rl.get_mouse_delta();
	let delta = rl.get_frame_time();

    // Camera speeds based on frame time
    let mut speed = CAMERA_MOVE_SPEED * delta;
    let rot_speed = CAMERA_ROTATION_SPEED * delta;

	let mut forward = (camera.target - camera.position).normalized();
	let up = camera.up;
	let right = forward.cross(up);

	let yaw = Quaternion::from_axis_angle(up, -mouse_delta.x * rot_speed);
	let pitch = Quaternion::from_axis_angle(right, -mouse_delta.y * rot_speed);

	forward = forward.rotate_by(yaw).rotate_by(pitch);

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