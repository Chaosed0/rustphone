use texture_packer::{texture::{Texture, memory_rgba8_texture::RGBA8}, *};
use crate::bsp::*;
use raylib::prelude::Vector2;

pub struct LightmapData
{
	pub lightmaps: Vec<LightmapPage>,
	pub surf_data: Vec<Option<SurfLightmapData>>,
}

pub struct LightmapPage
{
	pub bytes: Vec<u8>,
	pub width: u32,
	pub height: u32,
}

pub struct SurfLightmapData
{
	pub idx: usize,
	pub ofs: Vector2,
}

#[derive(Clone)]
struct SurfTex<'a>
{
	width: u32,
	height: u32,
	lightmap: &'a [u8],
}

struct DummyPixel;
impl texture_packer::texture::Pixel for DummyPixel {
	fn is_transparent(&self) -> bool {
		false
	}
	fn outline() -> Self {
		Self
	}
	fn transparency() -> Option<Self> {
		None
	}
}

impl Texture for SurfTex<'_>
{
	type Pixel = DummyPixel;

	fn height(&self) -> u32 { self.height }
	fn width(&self) -> u32 { self.width }
	fn get(&self, x: u32, y: u32) -> Option<Self::Pixel> {
		(x < self.width && y < self.height).then_some(DummyPixel)
	}
	fn set(&mut self, _: u32, _: u32, _: Self::Pixel) { }
}

pub fn pack_lightmaps(bsp: &Bsp) -> LightmapData
{
	let config = TexturePackerConfig {
		max_width: 4096,
		max_height: 4096,
		allow_rotation: false,
		texture_outlines: true,
		border_padding: 1,
		force_max_dimensions: false,
		..Default::default()
	};

	let mut packer = MultiTexturePacker::new_skyline(config);

	let mut frame_num = 0;
	let mut last_byte = 0;

	for (i, surf) in bsp.surfs.iter().enumerate()
	{
		if surf.lightofs == -1 { continue; }

		let width = surf.extent_x / 16 + 1;
		let height = (surf.extent_y / 16 + 1) * get_num_taps(&surf);
		let samples = width as usize * height as usize;
		let lightofs = (surf.lightofs * 3) as usize;
		let slice_end = lightofs + samples * 3;

		println!("Packing surf: {width} {height} {:?} {samples} {lightofs}. ", get_num_taps(&surf));

		let lmtex = SurfTex { width: width as u32, height: height as u32, lightmap: &bsp.lit_data[lightofs..slice_end] };
		packer.pack_own(i, lmtex).unwrap_or_else(|err| panic!("Couldn't pack {i:?}: {err:?}"));
		frame_num += 1;
		last_byte = last_byte.max(slice_end);
	}

	println!("Packed {frame_num} frames into {:?} lightmaps (last: {:?} count: {:?})", packer.get_pages().len(), last_byte, bsp.lit_data.len());

	let mut surf_data: Vec<Option<SurfLightmapData>> = std::iter::repeat_with(|| None).take(bsp.surfs.len()).collect();
	let mut lightmaps = Vec::<LightmapPage>::new();

	for (p, page) in packer.get_pages().iter().enumerate()
	{
		let mut lightmap = vec![0u8; (page.width() * page.height() * 3) as usize];

		println!("Building lightmap for page {p} ({:?}x{:?})", page.width(), page.height());

		for (f, frame) in page.get_frames()
		{
			let r = frame.frame;
			let surf = &bsp.surfs[*f];
			surf_data[*f] = Some(SurfLightmapData { idx: p, ofs: Vector2 { x: r.x as f32, y: r.y as f32 } });

			println!("  Inserting lightmap for surf {f} at {r:?}");

			for yofs in 0..r.h
			{
				let page_y = r.y + yofs;
				let page_start = ((page_y * page.width() + r.x) * 3) as usize;
				let page_end = (page_start as u32 + r.w * 3) as usize;

				let l_start = ((surf.lightofs as u32 + yofs * r.w) * 3) as usize;
				let l_end = (l_start as u32 + r.w * 3) as usize;
				
				lightmap[page_start..page_end].copy_from_slice(&bsp.lit_data[l_start..l_end]);
			}
		}

		lightmaps.push(LightmapPage { bytes: lightmap, width: page.width(), height: page.height() });
	}

	return LightmapData { lightmaps, surf_data };
}

fn get_num_taps(surf: &Surface) -> i16
{
	if surf.styles[1] == 255 { return 1; }
	if surf.styles[2] == 255 { return 2; }
	return 3;
}