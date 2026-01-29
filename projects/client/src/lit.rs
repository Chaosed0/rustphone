use texture_packer::{texture::Texture, *};
use crate::bsp::*;
use raylib::prelude::Vector2;

const PAGE_SIZE: u32 = 256;

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
		max_width: PAGE_SIZE,
		max_height: PAGE_SIZE,
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
		let height = surf.extent_y / 16 + 1;
		let samples = width as usize * height as usize;
		let lightofs = (surf.lightofs * 3) as usize;
		let slice_end = lightofs + samples * 3;

		//println!("Packing surf: {width} {height} {:?} {samples} {lightofs}. ", get_num_taps(&surf));

		let lmtex = SurfTex { width: width as u32, height: height as u32, lightmap: &bsp.lit_data[lightofs..slice_end] };
		packer.pack_own(i, lmtex).unwrap_or_else(|err| panic!("Couldn't pack {i:?}: {err:?}"));
		frame_num += 1;
        last_byte = lightofs + samples;
	}

	println!("Packed {frame_num} frames into {:?} lightmaps (last: {:?} count: {:?})", packer.get_pages().len(), last_byte, bsp.lit_data.len());

	let mut surf_data: Vec<Option<SurfLightmapData>> = std::iter::repeat_with(|| None).take(bsp.surfs.len()).collect();
	let mut lightmaps = Vec::<LightmapPage>::new();

    let page_count = packer.get_pages().len() as f32;
    let dimension = page_count.sqrt().ceil();
    let npot = dimension.log2().ceil().exp2() as u32;
    let lightmap_size = npot * PAGE_SIZE;

    println!("Building lightmap texture size {:?} (p: {:?} dim: {:?} npot: {:?})", lightmap_size, page_count, dimension, npot);

    let mut lightmap = vec![0u8; (lightmap_size * lightmap_size * 3) as usize];

	for (p, page) in packer.get_pages().iter().enumerate()
	{
        let p_basex = (p as f32).sqrt().floor();
        let p_base = p_basex.powi(2);
        let p_max = (p_basex + 1f32).powi(2);
        let p_x = (p_max - 1f32 - p as f32).min(p_basex);
        let p_y = (p as f32 - p_base).min(p_basex);

        let lm_pageofs = Vector2::new(p_x * PAGE_SIZE as f32, p_y * PAGE_SIZE as f32);
        let p_w = page.width();
        let p_h = page.height();

		println!("Building lightmap for page {p}, coord ({:?}, {:?}), dim ({:?}x{:?}). Frame count: {:?}", p_x, p_y, p_w, p_h, page.get_frames().len());

		for (f, frame) in page.get_frames()
		{
			let r = frame.frame;
			let surf = &bsp.surfs[*f];
			surf_data[*f] = Some(SurfLightmapData { idx: 0, ofs: lm_pageofs + Vector2 { x: r.x as f32, y: r.y as f32 } });

			//println!("  Inserting lightmap for surf {f} at {r:?}");

			for yofs in 0..r.h
			{
				let frame_y = r.y + yofs;
                let lm_frameofs = lm_pageofs + Vector2::new(r.x as f32, frame_y as f32);

				let frame_start = ((lm_frameofs.y * lightmap_size as f32 + lm_frameofs.x) * 3f32) as usize;
				let frame_end = (frame_start as u32 + r.w * 3) as usize;

				let l_start = ((surf.lightofs as u32 + yofs * r.w) * 3) as usize;
				let l_end = (l_start as u32 + r.w * 3) as usize;
				
                //println!("    Copy row {:?} (ofs {:?}): {:?}..{:?}->{:?}..{:?}", yofs, lm_frameofs, frame_start, frame_end, l_start, l_end);

				lightmap[frame_start..frame_end].copy_from_slice(&bsp.lit_data[l_start..l_end]);
			}
		}
	}

    lightmaps.push(LightmapPage { bytes: lightmap, width: lightmap_size, height: lightmap_size });

	return LightmapData { lightmaps, surf_data };
}

/*
fn get_num_taps(surf: &Surface) -> i16
{
	if surf.styles[1] == 255 { return 1; }
	if surf.styles[2] == 255 { return 2; }
	return 3;
}
*/
