mod transport;
use transport::Transport;

mod bsp;
use bsp::*;

mod palette;
use palette::PALETTE;

use std::ffi::c_void;
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
	let bsp = load_bsp("assets/box.bsp");

	let textures = bsp.textures.iter()
		.map(|mip_tex| {
			let image = image_from_pixels(mip_tex);
			return rl.load_texture_from_image(&thread, &image)
				.unwrap_or_else(|err| panic!("Could not generate texture from image: {err}"));
		})
		.collect::<Vec<Texture2D>>();

	let mut time = 0f32;

	rl.set_target_fps(60);

    while !rl.window_should_close() {
        transport.poll_messages(message_handler);
        gns_global.poll_callbacks();

		time += rl.get_frame_time();

        let mut d = rl.begin_drawing(&thread);

        d.clear_background(Color::WHITE);

		let tex_index = ((time * 0.5) % textures.len() as f32) as usize;
		d.draw_texture(&textures[tex_index], 0, 0, Color::WHITE);

        d.draw_text("Hello, world!", 12, 12, 20, Color::BLACK);
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

fn message_handler(_msg: Message) {
}
