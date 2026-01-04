use raylib::prelude::*;
use raylib::core::shaders::Shader;
use crate::bsp::{self, Bsp};
use crate::lit::{LightmapData, SurfLightmapData};
use glow::*;
use ::core::ffi::c_void;
use ::core::num::NonZeroU32;
use ::core::ptr::addr_of;
use ::core::fmt::Debug;
use std::fmt::Formatter;
use std::mem::offset_of;

pub struct BspRender
{
	gl: Context,
	data: Option<RenderData>,
}

struct RenderData
{
	vao: NativeVertexArray,
	vbo: NativeBuffer,
	ibo: NativeBuffer,
	cmds: Vec<DrawElementsIndirectCommand>,
	cmd_count: i32,
}

#[repr(C)]
struct GlVert
{
	pos: Vector3,
	col: Vector4,
	st: Vector4,
}

impl Debug for GlVert
{
	fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error>
	{
		formatter.write_fmt(format_args!("{:?}", self.pos)).unwrap();
		return Ok(());
	}
}

struct DrawElementsIndirectCommand
{
	count: i32,
	instanceCount: i32,
	firstIndex: i32,
	baseVertex: i32,
	baseInstance: u32,
}

impl BspRender
{
	pub fn new() -> Self
	{
		gl_loader::init_gl();

		let gl = unsafe { glow::Context::from_loader_function(|s| gl_loader::get_proc_address(s) as *const c_void) };

		return BspRender { gl, data: None };
	}

	pub fn build_buffers(&mut self, bsp: &Bsp, light_data: &LightmapData)
	{
		//let numverts = bsp.surfs.iter().map(|surf| surf.num_edges as i32).sum::<i32>();
		//let numtex = bsp.textures.len();
		let numtris = bsp.surfs.iter().map(|surf| (surf.num_edges.max(2) - 2) as i32).sum::<i32>();
		let mut verts = Vec::<GlVert>::new();
		let mut surf_vbo_map = Vec::<usize>::new();
		let mut indexes = vec![0u32; (numtris * 3) as usize];
		let mut cmds = Vec::<DrawElementsIndirectCommand>::new();

		for tex in &bsp.textures
		{
			//println!("TEX-CMD {:?}", tex.name);
			cmds.push(DrawElementsIndirectCommand { count: 0, instanceCount: 1, firstIndex: 0, baseVertex: 0, baseInstance: 0 })
		}

		for surf in &bsp.surfs
		{
			let tex_info = &bsp.tex_infos[surf.tex_info as usize];
			let tex_num = tex_info.tex_num as usize;
			let vcount =  ((surf.num_edges.max(2) - 2) * 3) as i32;
			cmds[tex_num].count += vcount;
			//println!("SURF has {:?} {:?} {:?}", tex_num, vcount, cmds[tex_num].count);
		}

		let mut sum = 0;
		for cmd in &mut cmds
		{
			cmd.firstIndex = sum;
			//println!("CMD has {:?} {:?}", cmd.firstIndex, cmd.count);
			sum += cmd.count;
		}

		for (i, surf) in bsp.surfs.iter().enumerate()
		{
			surf_vbo_map.push(verts.len());

			let surf_light_data = match &light_data.surf_data[i] { Some(v) => v, None => continue };
			let lightmap_data = &light_data.lightmaps[surf_light_data.idx];

			//println!("SURF: {:?} {:?}", surf.first_edge, surf.num_edges);
			//print!("   ");

			let tex_info = &bsp.tex_infos[surf.tex_info as usize];
			let texture = &bsp.textures[tex_info.tex_num as usize];

			if texture.name.starts_with("clip") || texture.name.starts_with("trigger") || texture.name.starts_with("skip") {
				continue;
			}

			for e in surf.first_edge..(surf.first_edge + surf.num_edges as i32)
			{
				let edge_index = bsp.surf_edges[e as usize];
				let edge = &bsp.edges[edge_index.abs() as usize];
				let v = if edge_index >= 0 { edge.v0 } else { edge.v1 };
				let vec = bsp.verts[v as usize];
				let s = (vec.dot(Vector3::new(tex_info.vec0.x, tex_info.vec0.y, tex_info.vec0.z)) + tex_info.vec0.w) / texture.width as f32;
				let t = (vec.dot(Vector3::new(tex_info.vec1.x, tex_info.vec1.y, tex_info.vec1.z)) + tex_info.vec1.w) / texture.height as f32;

				let light_s = ((s - surf.texture_mins0 as f32) / 16f32 + (surf_light_data.ofs.x + 0.5f32)) / lightmap_data.width as f32;
				let light_t = ((s - surf.texture_mins1 as f32) / 16f32 + (surf_light_data.ofs.y + 0.5f32)) / lightmap_data.height as f32;

				verts.push(GlVert { pos: vec, col: Color::WHITE.into(), st: Vector4::new(s, t, light_s, light_t) });

				/*
				print!("{:?} ({:?},{:?},{:?})", edge_index, vec.x, vec.y, vec.z);
				if e != surf.first_edge + surf.num_edges as i32 - 1 {
					print!("->");
				}
				*/
			}

			//println!("");
		}

		for (s, surf) in bsp.surfs.iter().enumerate()
		{
			let vbo_firstvert = surf_vbo_map[s] as u32;
			let tex_info = &bsp.tex_infos[surf.tex_info as usize];
			let cmd = &mut cmds[tex_info.tex_num as usize];
			for e in 2..surf.num_edges
			{
				indexes[cmd.firstIndex       as usize] = vbo_firstvert;
				indexes[(cmd.firstIndex + 1) as usize] = vbo_firstvert + e as u32;
				indexes[(cmd.firstIndex + 2) as usize] = vbo_firstvert + e as u32 - 1;

				//println!(" surf {:?}-{:?}: {:?} | {:?} {:?} {:?}", s, e, cmd.firstIndex, vbo_firstvert, vbo_firstvert + e as u32 - 1, vbo_firstvert + e as u32);

				cmd.firstIndex += 3;
			}
		}

		let mut sum = 0;
		for cmd in &mut cmds
		{
			cmd.firstIndex = sum;
			sum += cmd.count;
		}

		for (cmd, tex) in cmds.iter().zip(&bsp.textures)
		{
			println!("tex has {:?} cmds", cmd.count)
		}

		unsafe
		{
			let verts_u8: &[u8] = std::slice::from_raw_parts(verts.as_ptr() as *const u8, verts.len() * size_of::<GlVert>());
			let indexes_u8: &[u8] = std::slice::from_raw_parts(indexes.as_ptr() as *const u8, indexes.len() * size_of::<u32>());

			let vao = self.gl.create_vertex_array().unwrap();
			let vbo = self.gl.create_buffer().unwrap();
			let ibo = self.gl.create_buffer().unwrap();

			self.gl.bind_vertex_array(Some(vao));
			self.gl.bind_buffer(ARRAY_BUFFER, Some(vbo));
			self.gl.buffer_data_u8_slice(ARRAY_BUFFER, verts_u8, STATIC_DRAW);
			self.gl.bind_buffer(ELEMENT_ARRAY_BUFFER, Some(ibo));
			self.gl.buffer_data_u8_slice(ELEMENT_ARRAY_BUFFER, indexes_u8, STATIC_DRAW);

			self.gl.vertex_attrib_pointer_f32(0, 3, FLOAT, false, size_of::<GlVert>() as i32, offset_of!(GlVert, pos) as i32);
			self.gl.vertex_attrib_pointer_f32(1, 4, FLOAT, false, size_of::<GlVert>() as i32, offset_of!(GlVert, col) as i32);
			self.gl.vertex_attrib_pointer_f32(2, 4, FLOAT, false, size_of::<GlVert>() as i32, offset_of!(GlVert, st) as i32);
			self.gl.enable_vertex_array_attrib(vao, 0);
			self.gl.enable_vertex_array_attrib(vao, 1);
			self.gl.enable_vertex_array_attrib(vao, 2);

			self.data = Some(RenderData { vao, vbo, ibo, cmds, cmd_count: sum });
		}

		//println!("VERTS {:?}", verts);
		//println!("ELEMS {:?}", indexes);
	}

	pub fn is_ready(&self) -> bool
	{
		return match self.data { Some(_) => true, None => false };
	}

	pub fn render(&self, textures: &Vec<Texture2D>, lightmaps: &Vec<Texture2D>, bsp: &Bsp, light_data: &Vec<Option<SurfLightmapData>>, default_shader: &Shader, cutout_shader: &Shader, mvp: Matrix, time: f32)
	{
		let data = match &self.data { Some(v) => v, None => return };

		unsafe
		{
			let gl_default_shader = NonZeroU32::new(default_shader.id)
				.map(NativeProgram)
				.expect("Unable to create Shader object");

			let gl_cutout_shader = NonZeroU32::new(cutout_shader.id)
				.map(NativeProgram)
				.expect("Unable to create Shader object");

			//let mat_f32 = std::slice::from_raw_parts(addr_of!(mvp) as *const f32, 16);
			let mat_f32 =
			[
				mvp.m0, mvp.m1, mvp.m2, mvp.m3,
				mvp.m4, mvp.m5, mvp.m6, mvp.m7,
				mvp.m8, mvp.m9, mvp.m10, mvp.m11,
				mvp.m12, mvp.m13, mvp.m14, mvp.m15,
			];

			let mvp_d_loc = self.gl.get_uniform_location(gl_default_shader, "mvp");
			let mvp_c_loc = self.gl.get_uniform_location(gl_cutout_shader, "mvp");

			self.gl.bind_vertex_array(Some(data.vao));

			for (cmd, tex, bsptex) in itertools::izip!(&data.cmds, textures, &bsp.textures)
			{
				if cmd.count == 0 {
					continue
				}

				match bsptex.tex_type
				{
					bsp::TextureType::Cutout => {
						self.gl.use_program(Some(gl_cutout_shader));
						self.gl.uniform_matrix_4_f32_slice(mvp_c_loc.as_ref(), false, &mat_f32);
					}
					_ => {
						self.gl.use_program(Some(gl_default_shader));
						self.gl.uniform_matrix_4_f32_slice(mvp_d_loc.as_ref(), false, &mat_f32);
					}
				}

				let gl_tex = NonZeroU32::new(tex.id)
					.map(NativeTexture)
					.expect("Unable to create Texture object");

				self.gl.active_texture(0);
				self.gl.bind_texture(TEXTURE_2D, Some(gl_tex));
				self.gl.draw_elements(TRIANGLES, cmd.count, UNSIGNED_INT, cmd.firstIndex * size_of::<u32>() as i32);
			}

			self.gl.bind_texture(TEXTURE_2D, None);

			let gl_tex = NonZeroU32::new(textures[0].id)
				.map(NativeTexture)
				.expect("Unable to create Texture object");

			self.gl.active_texture(0);
			self.gl.bind_texture(TEXTURE_2D, Some(gl_tex));

			self.gl.use_program(None);
		}
	}
}

impl Drop for BspRender
{
	fn drop(&mut self)
	{
		gl_loader::end_gl();
	}
}