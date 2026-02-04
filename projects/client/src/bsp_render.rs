use raylib::prelude::*;
use crate::bsp::*;
use crate::lit::LightmapData;
use glow::*;
use ::core::ffi::c_void;
use ::core::num::NonZeroU32;
use ::core::fmt::Debug;
use std::fmt::Formatter;
use std::mem::offset_of;

type RlShader = raylib::core::shaders::Shader;

pub struct BspRender
{
	gl: Context,
	data: Option<RenderData>,
	lightgrid_data: Option<LightgridData>,
	skybox: Option<NativeTexture>,
	shaders: Option<ShaderSet>
}

struct RenderData
{
	vao: NativeVertexArray,
	vbo: NativeBuffer,
	ibo: NativeBuffer,
	cmds: Vec<DrawElementsIndirectCommand>,
	cmd_count: i32,
}

struct LightgridData
{
	sample_buffer: NativeBuffer,
}

struct ShaderSet
{
	default: Shader,
	cutout: Shader,
	skybox: Shader,
}

struct Shader
{
	id: NativeProgram,
	locs: ShaderLocationSet,
}

#[derive(Default)]
struct ShaderLocationSet
{
	mvp: Option<NativeUniformLocation>,
	texture: Option<NativeUniformLocation>,
	lightmap: Option<NativeUniformLocation>,
	skybox: Option<NativeUniformLocation>,
	eye_pos: Option<NativeUniformLocation>,
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

impl Shader
{
	fn new(shader: &RlShader) -> Shader
	{
		let gl_shader = NonZeroU32::new(shader.id)
			.map(NativeProgram)
			.expect("Unable to create Shader object");

		return Shader { id: gl_shader, locs: Default::default() };
	}

	fn _use(&self, gl: &Context)
	{
		unsafe { gl.use_program(Some(self.id)) };
	}

	fn load_mvp(&mut self, gl: &Context)
	{
		self.locs.mvp = self.load_loc(gl, "mvp");
	}

	fn load_eye_pos(&mut self, gl: &Context)
	{
		self.locs.eye_pos = self.load_loc(gl, "eyePos");
	}

	fn load_tex(&mut self, gl: &Context)
	{
		self.locs.texture = self.load_loc(gl, "tex");
	}

	fn load_lightmap(&mut self, gl: &Context)
	{
		self.locs.lightmap = self.load_loc(gl, "lightmap");
	}

	fn load_skybox(&mut self, gl: &Context)
	{
		self.locs.skybox = self.load_loc(gl, "skybox");
	}

	fn load_loc(&self, gl: &Context, name: &str) -> Option<NativeUniformLocation>
	{
		return unsafe { gl.get_uniform_location(self.id, name) };
	}
}

impl BspRender
{
	pub fn new() -> Self
	{
		gl_loader::init_gl();

		let gl = unsafe { glow::Context::from_loader_function(|s| gl_loader::get_proc_address(s) as *const c_void) };

		return BspRender { gl, data: None, lightgrid_data: None, skybox: None, shaders: None };
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

		for _ in &bsp.textures
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

			let surf_light_data = &light_data.surf_data[i];

			//println!("SURF: {:?} {:?}", surf.first_edge, surf.num_edges);
			//print!("   ");

			let tex_info = &bsp.tex_infos[surf.tex_info as usize];
			let texture = &bsp.textures[tex_info.tex_num as usize];

			for e in surf.first_edge..(surf.first_edge + surf.num_edges as i32)
			{
				let edge_index = bsp.surf_edges[e as usize];
				let edge = &bsp.edges[edge_index.abs() as usize];
				let v = if edge_index >= 0 { edge.v0 } else { edge.v1 };
				let vec = bsp.verts[v as usize];

				let projected = Vector2::new(vec.dot(tex_info.v0), vec.dot(tex_info.v1)) + tex_info.offset;
				let texture_size_inv = Vector2::new(1f32 / texture.width as f32, 1f32 / texture.height as f32);
				let uv = projected * texture_size_inv;

				let texture_mins = Vector2::new(surf.texture_mins0 as f32, surf.texture_mins1 as f32);
				let st = match surf_light_data {
					Some(data) => {
						let lightmap_data = &light_data.lightmaps[data.idx];
						let lightmap_size_inv = Vector2::new(1f32 / lightmap_data.width as f32, 1f32/ lightmap_data.height as f32);
						(data.ofs + 0.5f32 + (projected - texture_mins) * 1f32 / 16f32) * lightmap_size_inv
					},
					None => Vector2::ZERO
				};

				verts.push(GlVert { pos: vec, col: col_to_vec4(Color::WHITE), st: Vector4::new(uv.x, uv.y, st.x, st.y) });

				//println!("{:?} ({:?},{:?},{:?}) {:?} {:?} ({:?} {:?})", edge_index, vec.x, vec.y, vec.z, surf_light_data.ofs, (projected - texture_mins), st.x, st.y);
			}

			//println!("");
		}

		fn col_to_vec4(col: Color) -> Vector4 {
			return Vector4::new(col.r as f32 / 255f32, col.g as f32 / 255f32, col.b as f32 / 255f32, col.a as f32 / 255f32);
		}

		for (s, surf) in bsp.surfs.iter().enumerate()
		{
			let tex_info = &bsp.tex_infos[surf.tex_info as usize];
			let texture = &bsp.textures[tex_info.tex_num as usize];

			if texture.name.starts_with("clip") || texture.name.starts_with("trigger") || texture.name.starts_with("skip") {
				continue;
			}

			let vbo_firstvert = surf_vbo_map[s] as u32;
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

        /*
		for (cmd, tex) in cmds.iter().zip(&bsp.textures)
		{
			println!("tex has {:?} cmds", cmd.count)
		}
        */

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

	pub fn build_lightgrid_data(&mut self, lightgrid: &Lightgrid)
	{
		let grid_size = [lightgrid.header.grid_size[1], lightgrid.header.grid_size[2], lightgrid.header.grid_size[0]];
		let mut gl_samples = vec!(Default::default(); (grid_size[0] * grid_size[1] * grid_size[2]) as usize);
		println!("LEN {:?}", gl_samples.len());

		for leaf in &lightgrid.leafs {
			for z in 0..leaf.size[2] {
				for y in 0..leaf.size[1] {
					for x in 0..leaf.size[0] {
						let sample_idx = leaf.sample_start_idx + (x + y * leaf.size[0] + z * leaf.size[1] * leaf.size[0]) as usize;
						let sample = lightgrid.samples[sample_idx];
						let color = sample.samples[0].color;
						let gl_color = Vector4::new(color[0] as f32 / 255f32, color[1] as f32 / 255f32, color[2] as f32 / 255f32, if sample.used_styles > 0 { 1f32 } else { 0f32 });
						let gl_sample_pos = [ leaf.mins[1] + y, leaf.mins[2] + z, leaf.mins[0] + x ];
						let gl_sample_idx = (gl_sample_pos[0] + gl_sample_pos[1] * grid_size[0] + gl_sample_pos[2] * grid_size[0] * grid_size[1]) as usize;
						gl_samples[gl_sample_idx] = gl_color;
					}
				}
			}
		}

		unsafe {
			let samples_u8: &[u8] = std::slice::from_raw_parts(gl_samples.as_ptr() as *const u8, gl_samples.len() * size_of::<Vector4>());
			let ssbo = self.gl.create_buffer().unwrap();
			self.gl.bind_buffer(SHADER_STORAGE_BUFFER, Some(ssbo));
			self.gl.buffer_data_u8_slice(SHADER_STORAGE_BUFFER, samples_u8, STATIC_DRAW);
			self.gl.bind_buffer(SHADER_STORAGE_BUFFER, None);

			self.lightgrid_data = Some(LightgridData { sample_buffer: ssbo });
		}
	}

	pub fn load_shaders(&mut self, rl_default: &RlShader, rl_cutout: &RlShader, rl_skybox: &RlShader)
	{
		println!("Loading default shader");
		let mut default = Shader::new(rl_default);
		default._use(&self.gl);
		default.load_mvp(&self.gl);
		default.load_tex(&self.gl);
		default.load_lightmap(&self.gl);

		println!("Loading cutout shader");
		let mut cutout = Shader::new(rl_cutout);
		cutout._use(&self.gl);
		cutout.load_mvp(&self.gl);
		cutout.load_tex(&self.gl);
		cutout.load_lightmap(&self.gl);

		println!("Loading skybox shader");
		let mut skybox = Shader::new(rl_skybox);
		skybox._use(&self.gl);
		skybox.load_mvp(&self.gl);
		skybox.load_skybox(&self.gl);
		skybox.load_eye_pos(&self.gl);

		self.shaders = Some(ShaderSet { default, cutout, skybox });
	}

	pub fn load_skybox(&mut self, prefix: &str)
	{
		unsafe
		{
			let texture = self.gl.create_texture().unwrap();
			self.gl.bind_texture(TEXTURE_CUBE_MAP, Some(texture));
			let exe_dir_path = std::env::current_exe().expect("no exe path").parent().expect("PARENT").to_owned();

			const CUBEMAP_SIDES: [(&str, u32); 6] = [
				("up", TEXTURE_CUBE_MAP_POSITIVE_Y),
				("bk", TEXTURE_CUBE_MAP_NEGATIVE_X),
				("dn", TEXTURE_CUBE_MAP_NEGATIVE_Y),
				("ft", TEXTURE_CUBE_MAP_POSITIVE_X),
				("lf", TEXTURE_CUBE_MAP_NEGATIVE_Z),
				("rt", TEXTURE_CUBE_MAP_POSITIVE_Z)];

			for (side_name, gl_side) in CUBEMAP_SIDES
			{
				let name = exe_dir_path.join(format!("{prefix}_{side_name}.tga")).to_str().expect("No path to skybox!").to_owned();
				let image = Image::load_image(&name).expect("No skybox image");
				let size = image.get_pixel_data_size();
				let width = image.width;
				let height = image.height;
				let raw_image = image.to_raw();
				let image_data = std::slice::from_raw_parts::<u8>(raw_image.data as *const u8, size);
				let data = PixelUnpackData::Slice(Some(image_data));
				self.gl.tex_image_2d(gl_side, 0, RGB as i32, width, height, 0, RGB, UNSIGNED_BYTE, data);
			}

			self.gl.tex_parameter_i32(TEXTURE_CUBE_MAP, TEXTURE_MIN_FILTER, NEAREST as i32);
			self.gl.tex_parameter_i32(TEXTURE_CUBE_MAP, TEXTURE_MAG_FILTER, NEAREST as i32);
			self.gl.tex_parameter_i32(TEXTURE_CUBE_MAP, TEXTURE_WRAP_S, CLAMP_TO_EDGE as i32);
			self.gl.tex_parameter_i32(TEXTURE_CUBE_MAP, TEXTURE_WRAP_T, CLAMP_TO_EDGE as i32);
			self.gl.tex_parameter_i32(TEXTURE_CUBE_MAP, TEXTURE_WRAP_R, CLAMP_TO_EDGE as i32);

			self.skybox = Some(texture);
		}
	}

	pub fn is_ready(&self) -> bool
	{
		return match self.data { Some(_) => true, None => false };
	}

	pub fn render(&self, textures: &Vec<Texture2D>, lightmaps: &Vec<Texture2D>, bsp: &Bsp, mvp: Matrix, camera: Camera)
	{
		let data = match &self.data { Some(v) => v, None => return };

		unsafe
		{
			//let mat_f32 = std::slice::from_raw_parts(addr_of!(mvp) as *const f32, 16);
			let mat_f32 =
			[
				mvp.m0, mvp.m1, mvp.m2, mvp.m3,
				mvp.m4, mvp.m5, mvp.m6, mvp.m7,
				mvp.m8, mvp.m9, mvp.m10, mvp.m11,
				mvp.m12, mvp.m13, mvp.m14, mvp.m15,
			];

			assert!(lightmaps.len() == 1);
			let gl_lm = NonZeroU32::new(lightmaps[0].id)
				.map(NativeTexture)
				.expect("Unable to create Texture object");

			self.gl.bind_vertex_array(Some(data.vao));

			for (cmd, tex, bsptex) in itertools::izip!(&data.cmds, textures, &bsp.textures)
			{
				if cmd.count == 0 {
					continue
				}

				let shaders = self.shaders.as_ref().unwrap();

				let gl_tex = NonZeroU32::new(tex.id)
					.map(NativeTexture)
					.expect("Unable to create Texture object");

				match bsptex.tex_type
				{
					TextureType::Cutout => {
						shaders.cutout._use(&self.gl);
						self.bind_texture(TEXTURE0, &gl_tex, shaders.cutout.locs.texture, 0);
						self.bind_texture(TEXTURE1, &gl_lm, shaders.cutout.locs.lightmap, 1);
						self.gl.uniform_matrix_4_f32_slice(shaders.cutout.locs.mvp.as_ref(), false, &mat_f32);
					}
					TextureType::Sky => {
						shaders.skybox._use(&self.gl);
						self.bind_texture(TEXTURE0, &self.skybox.unwrap(), shaders.cutout.locs.skybox, 0);
						self.gl.uniform_matrix_4_f32_slice(shaders.skybox.locs.mvp.as_ref(), false, &mat_f32);
						self.gl.uniform_3_f32(shaders.skybox.locs.eye_pos.as_ref(), camera.position.x, camera.position.y, camera.position.z);
					}
					_ => {
						shaders.default._use(&self.gl);
						self.bind_texture(TEXTURE0, &gl_tex, shaders.cutout.locs.texture, 0);
						self.bind_texture(TEXTURE1, &gl_lm, shaders.cutout.locs.lightmap, 1);
						self.gl.uniform_matrix_4_f32_slice(shaders.cutout.locs.mvp.as_ref(), false, &mat_f32);
					}
				}

				self.gl.draw_elements(TRIANGLES, cmd.count, UNSIGNED_INT, cmd.firstIndex * size_of::<u32>() as i32);
			}

			self.gl.use_program(None);
		}
	}

	pub fn bind_lightgrid_data(&self)
	{
		unsafe {
			if let Some(lightgrid_data) = &self.lightgrid_data {
				self.gl.bind_buffer_base(SHADER_STORAGE_BUFFER, 3, Some(lightgrid_data.sample_buffer));
			}
		}
	}

	fn bind_texture(&self, unit: u32, tex: &NativeTexture, loc: Option<NativeUniformLocation>, index: i32)
	{
		unsafe
		{
			self.gl.active_texture(unit);
			self.gl.bind_texture(TEXTURE_2D, Some(*tex));
			self.gl.uniform_1_i32(loc.as_ref(), index);
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
