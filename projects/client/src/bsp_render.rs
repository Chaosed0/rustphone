use raylib::prelude::*;
use raylib::core::shaders::Shader;
use crate::bsp::Bsp;
use glow::*;
use ::core::ffi::c_void;
use ::core::num::NonZeroU32;
use ::core::ptr::addr_of;
use std::mem::offset_of;

pub struct BspRender
{
	gl: Context,
	data: Option<RenderData>,
}

struct RenderData
{
	vbo: NativeBuffer,
	ibo: NativeBuffer,
	cmds: Vec<DrawElementsIndirectCommand>,
}

#[repr(C)]
struct GlVert
{
	pos: Vector3,
	col: Vector4,
	st: Vector4,
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

	pub fn build_buffers(&mut self, bsp: &Bsp)
	{
		//let numverts = bsp.surfs.iter().map(|surf| surf.num_edges as i32).sum::<i32>();
		//let numtex = bsp.textures.len();
		let numtris = bsp.surfs.iter().map(|surf| (surf.num_edges.max(2) - 2) as i32).sum::<i32>();
		let mut verts = Vec::<GlVert>::new();
		let mut surf_vbo_map = Vec::<usize>::new();
		let mut indexes = vec![0u32; (numtris * 3) as usize];
		let mut cmds = Vec::<DrawElementsIndirectCommand>::new();

		for _ in &bsp.textures
		{
			cmds.push(DrawElementsIndirectCommand { count: 0, instanceCount: 1, firstIndex: 0, baseVertex: 0, baseInstance: 0 })
		}

		for surf in &bsp.surfs
		{
			let tex_info = &bsp.tex_infos[surf.tex_info as usize];
			cmds[tex_info.tex_num as usize].count += (surf.num_edges.max(2) - 2) as i32;
		}

		let mut sum = 0;
		for cmd in &mut cmds
		{
			cmd.firstIndex = sum;
			sum += cmd.count;
		}

		for surf in &bsp.surfs
		{
			surf_vbo_map.push(verts.len());

			for e in surf.first_edge..(surf.first_edge + surf.num_edges as i32)
			{
				let edge_index = bsp.surf_edges[e as usize];
				let edge = &bsp.edges[edge_index.abs() as usize];
				let v = if edge_index >= 0 { edge.v0 } else { edge.v1 };
				let vec = bsp.verts[v as usize];
				let tex_info = &bsp.tex_infos[surf.tex_info as usize];
				let s = vec.dot(Vector3::new(tex_info.vec1.x, tex_info.vec1.y, tex_info.vec1.z)) + tex_info.vec1.w;
				let t = vec.dot(Vector3::new(tex_info.vec2.x, tex_info.vec2.y, tex_info.vec2.z)) + tex_info.vec2.w;

				verts.push(GlVert { pos: vec, col: Color::WHITE.into(), st: Vector4::new(s, t, 0f32, 0f32) });
			}
		}

		for (i, surf) in bsp.surfs.iter().enumerate()
		{
			let vbo_firstvert = surf_vbo_map[i] as u32;
			let tex_info = &bsp.tex_infos[surf.tex_info as usize];
			let cmd = &mut cmds[tex_info.tex_num as usize];
			for i in 2..surf.num_edges
			{
				indexes[cmd.firstIndex       as usize] = vbo_firstvert;
				indexes[(cmd.firstIndex + 1) as usize] = vbo_firstvert + i as u32 - 1;
				indexes[(cmd.firstIndex + 2) as usize] = vbo_firstvert + i as u32;

				cmd.firstIndex += 3;
			}
		}

		let mut sum = 0;
		for cmd in &mut cmds
		{
			cmd.firstIndex = sum;
			sum += cmd.count;
		}

		unsafe
		{
			let verts_u8: &[u8] = std::slice::from_raw_parts(verts.as_ptr() as *const u8, verts.len() * size_of::<GlVert>());

			let vbo = self.gl.create_buffer().unwrap();
			self.gl.bind_buffer(ARRAY_BUFFER, Some(vbo));
			//self.gl.object_label(BUFFER, vbo.0.get(), Some("VBO"));
			self.gl.buffer_data_u8_slice(ARRAY_BUFFER, verts_u8, STATIC_DRAW);

			let indexes_u8: &[u8] = std::slice::from_raw_parts(indexes.as_ptr() as *const u8, indexes.len() * size_of::<u32>());
			let ibo = self.gl.create_buffer().unwrap();
			self.gl.bind_buffer(ARRAY_BUFFER, Some(ibo));
			//self.gl.object_label(BUFFER, ibo.0.get(), Some("IBO"));
			self.gl.buffer_data_u8_slice(ARRAY_BUFFER, indexes_u8, STATIC_DRAW);

			self.gl.bind_buffer(ARRAY_BUFFER, None);

			self.data = Some(RenderData { vbo, ibo, cmds });
		}
	}

	pub fn is_ready(&self) -> bool
	{
		return match self.data { Some(_) => true, None => false };
	}

	pub fn render(&self, textures: &Vec<Texture2D>, shader: &Shader, mvp: Matrix)
	{
		let data = match &self.data { Some(v) => v, None => return };

		unsafe
		{
			let gl_shader = NonZeroU32::new(shader.id)
				.map(NativeProgram)
				.expect("Unable to create Shader object");

			self.gl.use_program(Some(gl_shader));

			self.gl.bind_buffer(ELEMENT_ARRAY_BUFFER, Some(data.ibo));
			self.gl.bind_buffer(ARRAY_BUFFER, Some(data.vbo));
			self.gl.vertex_attrib_pointer_f32(0, 3, FLOAT, false, size_of::<GlVert>() as i32, offset_of!(GlVert, pos) as i32);
			self.gl.vertex_attrib_pointer_f32(1, 4, FLOAT, false, size_of::<GlVert>() as i32, offset_of!(GlVert, col) as i32);
			self.gl.vertex_attrib_pointer_f32(2, 4, FLOAT, false, size_of::<GlVert>() as i32, offset_of!(GlVert, st) as i32);

			let mat_f32: &[f32] = std::slice::from_raw_parts(addr_of!(mvp) as *const f32, 16);
			let mvp_loc = self.gl.get_uniform_location(gl_shader, "MVP");
			self.gl.uniform_matrix_4_f32_slice(mvp_loc.as_ref(), false, mat_f32);

			for (cmd, tex) in data.cmds.iter().zip(textures)
			{
				let gl_tex = NonZeroU32::new(tex.id)
					.map(NativeTexture)
					.expect("Unable to create Texture object");

				self.gl.active_texture(0);
				self.gl.bind_texture(TEXTURE_2D, Some(gl_tex));
				self.gl.draw_elements_instanced_base_vertex_base_instance(TRIANGLES, cmd.count, UNSIGNED_INT, cmd.firstIndex * size_of::<u32>() as i32, cmd.instanceCount, cmd.baseVertex, cmd.baseInstance);
			}

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