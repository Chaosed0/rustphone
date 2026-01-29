use raylib::core::math::*;
use raylib::prelude::*;
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;
use std::io::BufReader;
use std::str::FromStr;
use std::collections::HashSet;
use std::collections::HashMap;
use strum_macros::FromRepr;

const BSP2_VER: i32 = (('B' as i32) << 0) | (('S' as i32) << 8) | (('P' as i32) << 16) | (('2' as i32) << 24);
const LIT_VER: i32 = (('Q' as i32) << 0) | (('L' as i32) << 8) | (('I' as i32) << 16) | (('T' as i32) << 24);
const MAX_LIGHTMAPS: usize = 4;
const MAX_MAP_HULLS: usize = 4;
const NUM_AMBIENTS: usize = 4;

const TEXTURE_SPECIAL: i32 = 1;
const TEXTURE_MISSING: i32 = 2;

const SURF_PLANEBACK: i32 = 2;
const SURF_DRAWSKY: i32 = 4;
//const SURF_DRAWSPRITE: i32 = 8;
const SURF_DRAWTURB: i32 = 0x10;
const SURF_DRAWTILED: i32 = 0x20;
//const SURF_DRAWBACKGROUND: i32 = 0x40;
//const SURF_UNDERWATER: i32 = 0x80;
const SURF_NOTEXTURE: i32 = 0x100;
const SURF_DRAWFENCE: i32 = 0x200;
const SURF_DRAWLAVA: i32 = 0x400;
const SURF_DRAWSLIME: i32 = 0x800;
const SURF_DRAWTELE: i32 = 0x1000;
const SURF_DRAWWATER: i32 = 0x2000;

pub struct Bsp
{
	pub textures: Vec<Texture>,
	pub planes: Vec<Plane>,
	pub leafs: Vec<Leaf>,
	pub verts: Vec<Vector3>,
	pub edges: Vec<Edge>,
	pub nodes: Vec<Node>,
	pub tex_infos: Vec<TexInfo>,
	pub surfs: Vec<Surface>,
	pub surf_edges: Vec<i32>,
	pub clip_nodes: Vec<ClipNode>,
	pub mark_surfs: Vec<i32>,
	pub lit_data: Vec<u8>,
	pub vis_data: Vec<u8>,
	pub entities: Vec<Entity>,
	pub submodels: Vec<Model>,
	pub texofs: [usize; 7], // Offset into used_textures for each texture type
	pub used_textures: Vec<i32>,
}

#[derive(Default)]
struct BspHeader
{
	version: i32,
	entities: LumpHeader,
	planes: LumpHeader,
	mip_tex: LumpHeader,
	vertices: LumpHeader,
	visilist: LumpHeader,
	nodes: LumpHeader,
	tex_info: LumpHeader,
	faces: LumpHeader,
	lightmaps: LumpHeader,
	clip_nodes: LumpHeader,
	leaves: LumpHeader,
	mark_surfaces: LumpHeader,
	edges: LumpHeader,
	surf_edges: LumpHeader,
	models: LumpHeader,
}

#[derive(Default)]
struct LumpHeader
{
	offset: i32,
	size: i32
}

enum ModelType
{
	Brush,
	Alias,
	Sprite,
	NumTypes,
}

#[derive(Debug)]
pub struct Entity {
    pub map: HashMap<String, String>
}

struct Model
{
	mins: Vector3,
	maxs: Vector3,
	origin: Vector3,
	head_node: [i32; MAX_MAP_HULLS],
	visleafs: i32,
	first_face: i32,
	num_faces: i32
}

#[derive(Clone, Copy)]
pub enum TextureType
{
	Default = 0,
	Cutout = 1,
	Sky = 2,
	Lava = 3,
	Slime = 4,
	Tele = 5,
	Water = 6,
}

pub struct Plane
{
	pub normal: Vector3,
	pub dist: f32,
	pub p_type: u8,
	pub sign: u8,

	_pad0: u8,
	_pad1: u8
}

#[derive(FromRepr, Debug, Clone, Copy, PartialEq)]
#[repr(i32)]
pub enum LeafContents {
    Empty = -1,
    Solid = -2,
    Water = -3,
    Slime = -4,
    Lava = -5,
    Sky = -6,
    Origin = -7,
    Clip = -8
}

pub struct Leaf
{
	pub contents: LeafContents,
	pub visofs: i32, // -1 = no visibility info

	pub mins: [u32; 3], // for frustum culling
	pub maxs: [u32; 3],

	pub firstmarksurface : u32,
	pub nummarksurfaces : u32,

	pub ambient_level: [u8; NUM_AMBIENTS]
}

pub struct Node
{
	pub plane_index: u32,
	pub children: [i32;2], // Negative are -(leafs+1), not nodes
	pub mins: Vector3,
	pub maxs: Vector3,
	pub first_surf: u32,
	pub num_surf: u32
}

pub struct TexInfo
{
	pub v0: Vector3,
	pub v1: Vector3,
	pub offset: Vector2,
	pub tex_num: i32,
	pub flags: i32
}

pub struct Edge
{
	pub v0: u32,
	pub v1: u32
}

struct Face
{
	planenum: u32,
	side: i32,

	firstedge: i32,
	numedges: i32,
	texinfo: i32,

	styles: [u8; MAX_LIGHTMAPS],
	lightofs: i32, // start of [numstyles*surfsize] samples
}

pub struct Surface
{
	pub plane: u32,
	pub mins: Vector3,
	pub maxs: Vector3,
	pub flags: i32,
	
	pub first_edge: i32, // Lookup in model->surfedges, negative are backwards
	pub num_edges: i16,

	pub extent_x: i16,
	pub extent_y: i16,

	pub styles: [u8;MAX_LIGHTMAPS],
	pub lightofs: i32, // Index into lightdata, [numstyles*surfsize] samples

	pub texture_mins0: i32,
	pub texture_mins1: i32,
	pub tex_info: u32
}

pub struct ClipNode
{
	pub plane_index: i32,
	pub children: [i32;2] // negatives are contents
}

struct Hull
{
	clip_nodes: Vec<u32>,
	planes: Vec<u32>,
	first_clip_node: i32,
	last_clip_node: i32,
	clip_mins: Vector3,
	clip_maxs: Vector3
}

pub struct Texture
{
	pub name: String,
	pub width: u32,
	pub height: u32,
	pub offset1: u32,
	pub offset2: u32,
	pub offset4: u32,
	pub offset8: u32,
	pub tex_type: TextureType,
	pub pixels: Vec<u8>,
}

pub fn load_bsp(filename: &str) -> Bsp
{
	let exe_path = std::env::current_exe().expect("no exe path");
	let path = exe_path.parent().expect("no parent!").join(filename);
	let file = File::open(&path).unwrap_or_else(|err| panic!("{err}: couldn't open map {path:?}!"));
	let mut reader = BufReader::new(file);
	let mut buf = vec![0u8; 128];

	let mut header: BspHeader = Default::default();
	header.version = read_i32(&mut reader, &mut buf);
	header.entities = read_dir_entry(&mut reader, &mut buf);
	header.planes = read_dir_entry(&mut reader, &mut buf);
	header.mip_tex = read_dir_entry(&mut reader, &mut buf);
	header.vertices = read_dir_entry(&mut reader, &mut buf);
	header.visilist = read_dir_entry(&mut reader, &mut buf);
	header.nodes = read_dir_entry(&mut reader, &mut buf);
	header.tex_info = read_dir_entry(&mut reader, &mut buf);
	header.faces = read_dir_entry(&mut reader, &mut buf);
	header.lightmaps = read_dir_entry(&mut reader, &mut buf);
	header.clip_nodes = read_dir_entry(&mut reader, &mut buf);
	header.leaves = read_dir_entry(&mut reader, &mut buf);
	header.mark_surfaces = read_dir_entry(&mut reader, &mut buf);
	header.edges = read_dir_entry(&mut reader, &mut buf);
	header.surf_edges = read_dir_entry(&mut reader, &mut buf);
	header.models = read_dir_entry(&mut reader, &mut buf);

	println!("Loading bsp {:?}", filename);
	println!("  version: {:?} ({:?})", header.version, BSP2_VER);
	println!("  entities: {:?} {:?}", header.entities.offset, header.entities.size);
	println!("  planes: {:?} {:?}", header.planes.offset, header.planes.size);
	println!("  mip_tex: {:?} {:?}", header.mip_tex.offset, header.mip_tex.size);
	println!("  vertices: {:?} {:?}", header.vertices.offset, header.vertices.size);
	println!("  visilist: {:?} {:?}", header.visilist.offset, header.visilist.size);
	println!("  tex_info: {:?} {:?}", header.tex_info.offset, header.tex_info.size);
	println!("  nodes: {:?} {:?}", header.nodes.offset, header.nodes.size);
	println!("  faces: {:?} {:?}", header.faces.offset, header.faces.size);
	println!("  lightmaps: {:?} {:?}", header.lightmaps.offset, header.lightmaps.size);
	println!("  clip_nodes: {:?} {:?}", header.clip_nodes.offset, header.clip_nodes.size);
	println!("  leaves: {:?} {:?}", header.leaves.offset, header.leaves.size);
	println!("  mark_surfaces: {:?} {:?}", header.mark_surfaces.offset, header.mark_surfaces.size);
	println!("  edges: {:?} {:?}", header.edges.offset, header.edges.size);
	println!("  surf_edges: {:?} {:?}", header.surf_edges.offset, header.surf_edges.size);
	println!("  models: {:?} {:?}", header.models.offset, header.models.size);

	let vertexes = read_verts(header.vertices, &mut reader, &mut buf);
	let edges = read_edges(header.edges, &mut reader, &mut buf);
	let surf_edges = read_surf_edges(header.surf_edges, &mut reader, &mut buf);
	let textures = read_textures(header.mip_tex, &mut reader, &mut buf);
	let lit_data = read_lighting(header.lightmaps, &mut reader, &mut buf, &path);
	let planes = read_planes(header.planes, &mut reader, &mut buf);
	let tex_infos= read_texinfo(header.tex_info, &mut reader, &mut buf, &textures);
	let surfaces = read_faces(header.faces, &mut reader, &mut buf, &tex_infos, &textures, &vertexes, &surf_edges, &edges);
	let mark_surfaces = read_marksurfaces(header.mark_surfaces, &mut reader, &mut buf, surfaces.len() as i32);
	let vis_data = read_vis(header.visilist, &mut reader);
	let leafs = read_leafs(header.leaves, &mut reader, &mut buf);
	let nodes = read_nodes(header.nodes, &mut reader, &mut buf);
	let clip_nodes = read_clip_nodes(header.clip_nodes, &mut reader, &mut buf);
	let entities = read_entities(header.entities, &mut reader, &mut buf);
	let submodels = read_submodels(header.models, &mut reader, &mut buf);
	let (texofs, used_textures) = build_used_textures(&surfaces, &textures, &tex_infos);

	return Bsp {
		textures,
		planes,
		leafs,
		verts: vertexes,
		edges,
		nodes,
		surf_edges,
		tex_infos,
		surfs: surfaces,
		clip_nodes,
		mark_surfs: mark_surfaces,
		entities,
		lit_data,
		vis_data,
		submodels,
		texofs,
		used_textures,
	};
}

fn read_verts(header: LumpHeader, reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> Vec<Vector3>
{
	reader.seek(std::io::SeekFrom::Start(header.offset as u64))
		.unwrap_or_else(|err| panic!("{err}: Invalid vert offset {:?}", header.offset));
	let mut verts = Vec::<Vector3>::new();
	let count = header.size as usize / size_of::<Vector3>();

	for _ in 0..count
	{
		verts.push(read_vec3(reader, buf));
	}

	return verts;
}

fn read_edges(header: LumpHeader, reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> Vec<Edge>
{
	reader.seek(std::io::SeekFrom::Start(header.offset as u64))
		.unwrap_or_else(|err| panic!("{err}: Invalid edge offset {:?}", header.offset));

	let mut edges = Vec::<Edge>::new();
	let count = header.size as usize / size_of::<Edge>();

	for _ in 0..count
	{
		edges.push(Edge { v0: read_u32(reader, buf), v1: read_u32(reader, buf) });
	}

	return edges;
}

fn read_surf_edges(header: LumpHeader, reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> Vec<i32>
{
	reader.seek(std::io::SeekFrom::Start(header.offset as u64))
		.unwrap_or_else(|err| panic!("{err}: Invalid surf_edge offset {:?}", header.offset));

	let mut surf_edges = Vec::<i32>::new();
	let count = header.size as usize / size_of::<i32>();

	println!("Reading {count} surf edges");

	for _ in 0..count
	{
		let surf_edge = read_i32(reader, buf);
		surf_edges.push(surf_edge);
	}

	return surf_edges;
}

fn read_textures(header: LumpHeader, reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> Vec<Texture>
{
	if header.size == 0
	{
		println!("WARNING: No textures found in the bsp file!");
	}

	reader.seek(std::io::SeekFrom::Start(header.offset as u64))
		.unwrap_or_else(|err| panic!("{err}: Invalid texture offset {:?}", header.offset));

	let num_tex = read_i32(reader, buf);
	let mut dataofs = Vec::<i32>::new();

	println!("Reading {:?} textures", num_tex);

	for _ in 0..num_tex
	{
		let data_offset = read_i32(reader, buf);
		dataofs.push(data_offset);
	}

	let mut mip_texs = Vec::<Texture>::new();

	for data_offset in dataofs
	{
		if data_offset < 0 {
			// Still must add a texture, this indicates the missing texture
			mip_texs.push(Texture { name: "MISSING".into(), width: 0, height: 0, offset1: 0, offset2: 0, offset4: 0, offset8: 0, tex_type: TextureType::Default, pixels: vec![0;0]});
			continue;
		}

		let offset = header.offset + data_offset;

		println!("Reading texture from offset {offset} ({data_offset})");

		reader.seek(std::io::SeekFrom::Start(offset as u64))
			.unwrap_or_else(|err| panic!("{err}: Invalid mip_tex offset {offset}"));

		let name = read_string16(reader, buf);
		let width = read_u32(reader, buf);
		let height = read_u32(reader, buf);
		let offset1 = read_u32(reader, buf);
		let offset2 = read_u32(reader, buf);
		let offset4 = read_u32(reader, buf);
		let offset8 = read_u32(reader, buf);
		let tex_type = texture_type_from_name(&name);

		let pixel_count = width * height;
		let mut pixels = vec![0u8; pixel_count as usize];
		reader.read_exact(&mut pixels)
			.unwrap_or_else(|err| panic!("Failed to read pixels for texture {name}: {err}"));

		if width == 0 || height == 0 {
			println!("WARNING: Zero sized texture {:?}!", name);
		}

		println!("Read texture {name}, width {width}, height {height}");

		mip_texs.push(Texture { name, width, height, offset1, offset2, offset4, offset8, tex_type, pixels });
	}

	return mip_texs;
}

fn read_lighting(header: LumpHeader, reader: &mut BufReader<File>, buf: &mut Vec<u8>, path: &Path) -> Vec<u8>
{
	let lit_filename = path.with_extension("lit");

	if lit_filename.exists()
	{
		let file = File::open(&lit_filename).unwrap_or_else(|err| panic!("{err}: couldn't open lit file at {lit_filename:?}!"));
		let mut lit_reader = BufReader::new(file);

		let lit_header = read_i32(&mut lit_reader, buf);
		if lit_header != LIT_VER
		{
			panic!("Header {lit_header} in lit file {lit_filename:?} doesn't match expected ({LIT_VER})!");
		}

		let lit_version = read_i32(&mut lit_reader, buf);
		if lit_version != 1
		{
			panic!("Version {lit_version} in lit file {lit_filename:?} doesn't match expected (1)!");
		}

		println!("Loaded lit file {lit_filename:?}");
		let mut lit_data = Vec::<u8>::new();
		lit_reader.read_to_end(&mut lit_data).unwrap_or_else(|err| panic!("Couldn't read lit file bytes: {err}"));
		return lit_data;
	}
	else
	{
		let mut lit_data = vec![0u8; header.size as usize];
		reader.read_exact(&mut lit_data).unwrap_or_else(|err| panic!("Couldn't read lighting data from bsp: {err}"));
		return lit_data;
	}
}

fn read_planes(header: LumpHeader, reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> Vec<Plane>
{
	reader.seek(std::io::SeekFrom::Start(header.offset as u64))
		.unwrap_or_else(|err| panic!("{err}: Invalid plane offset {:?}", header.offset));
	let mut planes = Vec::<Plane>::new();
	let count = header.size as usize / size_of::<Plane>();

	for _ in 0..count
	{
		let normal = read_vec3(reader, buf);
		let bits = if normal.x < 0f32 { 1 << 0 } else { 0 } |
			if normal.y < 0f32 { 1 << 1 } else { 0 } |
			if normal.z < 0f32 { 1 << 2 } else { 0 };

		let dist = read_f32(reader, buf);
		let p_type = read_i32(reader, buf) as u8;

		planes.push(Plane { normal, dist, p_type, sign: bits, _pad0: 0, _pad1: 0 });
	}

	return planes;
}

fn read_texinfo(header: LumpHeader, reader: &mut BufReader<File>, buf: &mut Vec<u8>, texs: &Vec<Texture>) -> Vec<TexInfo>
{
	reader.seek(std::io::SeekFrom::Start(header.offset as u64))
		.unwrap_or_else(|err| panic!("{err}: Invalid tex_info offset {:?}", header.offset));
	let mut tex_infos = Vec::<TexInfo>::new();
	let count = header.size as usize / size_of::<TexInfo>();
	let mut missing = 0;

	for _ in 0..count
	{
		let v0 = read_vec3(reader, buf);
		let ofs_x = read_f32(reader, buf);
		let v1 = read_vec3(reader, buf);
		let ofs_y = read_f32(reader, buf);

		let mip_tex = read_i32(reader, buf);
		let mut flags = read_i32(reader, buf);

		if mip_tex as usize >= texs.len() || texs[mip_tex as usize].width == 0 || texs[mip_tex as usize].height == 0
		{
			println!("MISSING TEXTURE: {:?}", mip_tex as usize);
			flags = flags | TEXTURE_MISSING;
			missing += 1;
		}

		tex_infos.push(TexInfo { v0, v1, offset: Vector2::new(ofs_x, ofs_y), tex_num: mip_tex, flags });
	}

	if count > 0 && missing > 0
	{
		println!("WARNING: Missing {missing} textures in BSP file");
	}

	return tex_infos;
}

fn read_faces(header: LumpHeader, reader: &mut BufReader<File>, buf: &mut Vec<u8>, tex_info: &Vec<TexInfo>, textures: &Vec<Texture>, vertexes: &Vec<Vector3>, surf_edges: &Vec<i32>, edges: &Vec<Edge>) -> Vec<Surface>
{
	reader.seek(std::io::SeekFrom::Start(header.offset as u64))
		.unwrap_or_else(|err| panic!("{err}: Invalid face offset {:?}", header.offset));
	let mut faces = Vec::<Face>::new();
	let count = header.size as usize / size_of::<Face>();

	for _ in 0..count
	{
		let planenum = read_u32(reader, buf);
		let side = read_i32(reader, buf);
		let firstedge = read_i32(reader, buf);
		let numedges = read_i32(reader, buf);
		let texinfo = read_i32(reader, buf);

		let styles0 = read_u8(reader, buf);
		let styles1 = read_u8(reader, buf);
		let styles2 = read_u8(reader, buf);
		let styles3 = read_u8(reader, buf);
		let lightofs = read_i32(reader, buf);

		faces.push(Face { planenum, side, firstedge, numedges, texinfo, styles: [styles0, styles1, styles2, styles3], lightofs});
	}

	let mut surfs = Vec::<Surface>::new();

	for face in faces
	{
		if face.numedges < 3
		{
			println!("WARNING: Bad edge count in face: {:?}", face.numedges)
		}

		let mut flags = if face.side > 0 { SURF_PLANEBACK } else { 0 };

		let tex_info = &tex_info[face.texinfo as usize];

		// CALC TEXTURE AND SURFACE BOUNDS
		let mut tmin0 = Option::<f32>::None;
		let mut tmax0 = Option::<f32>::None;
		let mut tmin1 = Option::<f32>::None;
		let mut tmax1 = Option::<f32>::None;
		let mut mins = Option::<Vector3>::None;
		let mut maxs = Option::<Vector3>::None;

		for i in 0..face.numedges
		{
			let surf_edge = surf_edges[(face.firstedge + i) as usize];
			let edge = &edges[surf_edge.abs() as usize];
			let vert_index = if surf_edge >= 0 { edge.v0 } else { edge.v1 };
			let vert = vertexes[vert_index as usize];

			// From Ironwail:
			/* The following calculation is sensitive to floating-point
			* precision.  It needs to produce the same result that the
			* light compiler does, because R_BuildLightMap uses surf->
			* extents to know the width/height of a surface's lightmap,
			* and incorrect rounding here manifests itself as patches
			* of "corrupted" looking lightmaps.
			* Most light compilers are win32 executables, so they use
			* x87 floating point.  This means the multiplies and adds
			* are done at 80-bit precision, and the result is rounded
			* down to 32-bits and stored in val.
			* Adding the casts to double seems to be good enough to fix
			* lighting glitches when Quakespasm is compiled as x86_64
			* and using SSE2 floating-point.  A potential trouble spot
			* is the hallway at the beginning of mfxsp17.  -- ericw
			*/
			let val0 =
				(vert.x as f64 * tex_info.v0.x as f64) +
				(vert.y as f64 * tex_info.v0.y as f64) +
				(vert.z as f64 * tex_info.v0.z as f64) +
				tex_info.offset.x as f64;

			let val1 =
				(vert.x as f64 * tex_info.v1.x as f64) +
				(vert.y as f64 * tex_info.v1.y as f64) +
				(vert.z as f64 * tex_info.v1.z as f64) +
				tex_info.offset.y as f64;

			tmin0 = match tmin0 {
				None => Some(val0 as f32),
				Some(v) => Some(v.min(val0 as f32))
			};

			tmax0 = match tmax0 {
				None => Some(val0 as f32),
				Some(v) => Some(v.max(val0 as f32))
			};

			tmin1 = match tmin1 {
				None => Some(val1 as f32),
				Some(v) => Some(v.min(val1 as f32))
			};

			tmax1 = match tmax1 {
				None => Some(val1 as f32),
				Some(v) => Some(v.max(val1 as f32))
			};

			mins = match mins {
				None => Some(vert),
				Some(v) => Some(v.min(vert))
			};

			maxs = match maxs {
				None => Some(vert),
				Some(v) => Some(v.max(vert))
			};
		}

		let bmin0 = 16 * (tmin0.unwrap() / 16f32).floor() as i32;
		let bmax0 = 16 * (tmax0.unwrap() / 16f32).ceil() as i32;
		let texture_mins0 = bmin0;
		let extent0 = (bmax0 - bmin0) as i16;

		let bmin1 = 16 * (tmin1.unwrap() / 16f32).floor() as i32;
		let bmax1 = 16 * (tmax1.unwrap() / 16f32).ceil() as i32;
		let texture_mins1 = bmin1;
		let extent1 = (bmax1 - bmin1) as i16;

		// END CALC SURFACE BOUNDS

		let tex: &Texture = &textures[tex_info.tex_num as usize];

		match tex.tex_type
		{
			TextureType::Sky => {
				flags = flags | (SURF_DRAWSKY | SURF_DRAWTILED);
			}
			TextureType::Lava => {
				flags = flags | (SURF_DRAWTURB | SURF_DRAWLAVA);
			}
			TextureType::Slime => {
				flags = flags | (SURF_DRAWTURB | SURF_DRAWSLIME);
			}
			TextureType::Tele => {
				flags = flags | (SURF_DRAWTURB | SURF_DRAWTELE);
			}
			TextureType::Water => {
				flags = flags | (SURF_DRAWTURB | SURF_DRAWWATER);
			}
			TextureType::Cutout => {
				flags = flags | SURF_DRAWFENCE;
			}
			TextureType::Default => ()
		};

		if tex_info.flags & TEXTURE_MISSING > 0
		{
			flags = flags | SURF_NOTEXTURE;
		}

		surfs.push(Surface {
			plane: face.planenum,
			mins: mins.unwrap(),
			maxs: maxs.unwrap(),
			flags,
			first_edge: face.firstedge,
			num_edges: face.numedges as i16,
			extent_x: extent0,
			extent_y: extent1,
			lightofs: face.lightofs,
			tex_info: face.texinfo as u32,
			styles: face.styles,
			texture_mins0,
			texture_mins1
		})
	}

	return surfs;
}

fn read_marksurfaces(header: LumpHeader, reader: &mut BufReader<File>, buf: &mut Vec<u8>, max_surfcount: i32) -> Vec<i32>
{
	reader.seek(std::io::SeekFrom::Start(header.offset as u64))
		.unwrap_or_else(|err| panic!("{err}: Invalid marksurfaces offset {:?}", header.offset));
	let mut mark_surfaces = Vec::<i32>::new();
	let count = header.size as usize / size_of::<i32>();

	for _ in 0..count
	{
		let mark_surface = read_i32(reader, buf);

		if mark_surface >= max_surfcount {
			panic!("Bad surface number {mark_surface} (max {max_surfcount})");
		}

		mark_surfaces.push(mark_surface);
	}

	return mark_surfaces;
}

fn read_vis(header: LumpHeader, reader: &mut BufReader<File>) -> Vec<u8>
{
	reader.seek(std::io::SeekFrom::Start(header.offset as u64))
		.unwrap_or_else(|err| panic!("Invalid vis offset {:?}: {err}", header.offset));
	let mut vis = vec![0u8; header.size as usize];
	reader.read_exact(&mut vis)
		.unwrap_or_else(|err| panic!("Could not read vis bytes length {:?}: {err}", header.size));
	return vis;
}

fn read_leafs(header: LumpHeader, reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> Vec<Leaf>
{
	reader.seek(std::io::SeekFrom::Start(header.offset as u64))
		.unwrap_or_else(|err| panic!("{err}: Invalid marksurfaces offset {:?}", header.offset));
	let mut leafs = Vec::<Leaf>::new();
	let count = header.size as usize / size_of::<Leaf>();

	for _ in 0..count
	{
        let contents_i = read_i32(reader, buf); 
		let contents = LeafContents::from_repr(contents_i).unwrap();
		let visofs = read_i32(reader, buf);
		let mins = [read_u32(reader, buf), read_u32(reader, buf), read_u32(reader, buf)];
		let maxs = [read_u32(reader, buf), read_u32(reader, buf), read_u32(reader, buf)];
		let firstmarksurface = read_u32(reader, buf);
		let nummarksurfaces = read_u32(reader, buf);
		let ambient_level = [read_u8(reader, buf), read_u8(reader, buf), read_u8(reader, buf), read_u8(reader, buf)];

		leafs.push(Leaf { contents, visofs, mins, maxs, firstmarksurface, nummarksurfaces, ambient_level })
	}

	return leafs;
}

fn read_nodes(header: LumpHeader, reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> Vec<Node>
{
	reader.seek(std::io::SeekFrom::Start(header.offset as u64))
		.unwrap_or_else(|err| panic!("{err}: Invalid nodes offset {:?}", header.offset));
	let mut nodes = Vec::<Node>::new();
	let count = header.size as usize / size_of::<Node>();

	for _ in 0..count
	{
		let plane_index = read_u32(reader, buf);
		let child0 = read_i32(reader, buf);
		let child1 = read_i32(reader, buf);
		let mins = read_vec3(reader, buf);
		let maxs = read_vec3(reader, buf);
		let first_surf = read_u32(reader, buf);
		let num_surf = read_u32(reader, buf);

		nodes.push(Node { plane_index, children: [child0, child1], mins, maxs, first_surf, num_surf });
	}

	return nodes;
}

fn read_clip_nodes(header: LumpHeader, reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> Vec<ClipNode>
{
	reader.seek(std::io::SeekFrom::Start(header.offset as u64))
		.unwrap_or_else(|err| panic!("{err}: Invalid clip nodes offset {:?}", header.offset));
	let mut nodes = Vec::<ClipNode>::new();
	let count = header.size as usize / size_of::<ClipNode>();

	for _ in 0..count
	{
		let plane_index = read_i32(reader, buf);
		let child0 = read_i32(reader, buf);
		let child1 = read_i32(reader, buf);

		nodes.push(ClipNode { plane_index, children: [child0, child1] });
	}

	return nodes;
}

fn read_entities(header: LumpHeader, reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> Vec<Entity>
{
	reader.seek(std::io::SeekFrom::Start(header.offset as u64))
		.unwrap_or_else(|err| panic!("Invalid entities offset {:?}: {err}", header.offset));

    #[derive(PartialEq, Debug)]
    enum ParserState {
        LookingForEntity,
        LookingForKey,
        InsideKey,
        LookingForValue,
        InsideValue,
    }

    let mut state = ParserState::LookingForEntity;
    let mut entity = Entity { map: HashMap::new() };
    let mut key = vec!();
    let mut value = vec!();
    let mut entities = vec!();

    for i in 0..(header.size as usize) {
        let byte = read_u8(reader, buf);

        if state == ParserState::LookingForEntity {
            if byte == '{' as u8 {
                state = ParserState::LookingForKey;
            } else if byte == '\0' as u8 {
                assert!(i == (header.size - 1) as usize);
            } else if !char::is_whitespace(byte.into()) {
                panic!("Found unexpected character {:?} while outside entity", char::from(byte));
            }
        } else if state == ParserState::LookingForKey {
            if byte == '}' as u8 {
                entities.push(entity);
                entity = Entity { map: HashMap::new() };
                state = ParserState::LookingForEntity;
            } else if byte == '\"' as u8 {
                state = ParserState::InsideKey;
            } else if !char::is_whitespace(byte.into()) {
                panic!("Found unexpected character {:?} while looking for key", char::from(byte));
            }
        } else if state == ParserState::InsideKey {
            if byte == '\"' as u8 {
                state = ParserState::LookingForValue;
            } else {
                key.push(byte);
            }
        } else if state == ParserState::LookingForValue {
            if byte == '\"' as u8 {
                state = ParserState::InsideValue;
            } else if !char::is_whitespace(byte.into()) {
                panic!("Found unexpected character {:?} while looking for key", char::from(byte));
            }
        } else if state == ParserState::InsideValue {
            if byte == '\"' as u8 {
                let key_str = String::from_utf8(key).unwrap();
                let value_str = String::from_utf8(value).unwrap();
                entity.map.insert(key_str, value_str);

                key = vec!();
                value = vec!();

                state = ParserState::LookingForKey;
            } else {
                value.push(byte);
            }
        }
    }

	return entities;
}

fn read_submodels(header: LumpHeader, reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> Vec<Model>
{
	reader.seek(std::io::SeekFrom::Start(header.offset as u64))
		.unwrap_or_else(|err| panic!("{err}: Invalid submodels offset {:?}", header.offset));
	let mut nodes = Vec::<Model>::new();
	let count = header.size as usize / size_of::<Model>();

	for _ in 0..count
	{
		let mins = read_vec3(reader, buf);
		let maxs = read_vec3(reader, buf);
		let origin = read_vec3(reader, buf);
		let headnode0 = read_i32(reader, buf);
		let headnode1 = read_i32(reader, buf);
		let headnode2 = read_i32(reader, buf);
		let headnode3 = read_i32(reader, buf);
		let visleafs = read_i32(reader, buf);
		let first_face = read_i32(reader, buf);
		let num_faces = read_i32(reader, buf);

		nodes.push(Model { mins, maxs, origin, head_node: [headnode0, headnode1, headnode2, headnode3], visleafs, first_face, num_faces });
	}

	return nodes;
}

fn build_used_textures(surfs: &Vec<Surface>, textures: &Vec<Texture>, tex_infos: &Vec<TexInfo>) -> ([usize; 7], Vec<i32>)
{
	let mut used_set = HashSet::<i32>::new();
	let mut texofs = [0usize; 7];

	for surf in surfs
	{
		let tex_info = &tex_infos[surf.tex_info as usize];
		let tex = &textures[tex_info.tex_num as usize];
		if used_set.insert(tex_info.tex_num)
		{
			texofs[tex.tex_type as usize] += 1;
			println!("Haven't seen {:?} before, adding 1 to {:?}", tex.name, tex.tex_type as i32);
		}
	}

	let mut sum = 0;
	for i in 0..7
	{
		let ofs = texofs[i];
		texofs[i] = sum;
		sum += ofs;
	}

	let mut used_textures = vec![0i32; sum as usize];
	let mut cur_ofs = texofs.clone();

	for (i, tex) in textures.iter().enumerate()
	{
		let idx = i as i32;
		if used_set.contains(&idx)
		{
			used_textures[cur_ofs[tex.tex_type as usize]] = idx;
			cur_ofs[tex.tex_type as usize] += 1;
		}
	}

	return (texofs, used_textures);
}

fn texture_type_from_name(name: &String) -> TextureType
{
	if name.starts_with("*")
	{
		if name[1..5] == *"lava" {
			return TextureType::Lava;
		}
		else if name[1..6] == *"slime" {
			return TextureType::Slime;
		}
		else if name[1..5] == *"tele" {
			return TextureType::Tele;
		}

		return TextureType::Water;
	}

	if name.starts_with("{") {
		return TextureType::Cutout;
	}

	if name.starts_with("sky") {
		return TextureType::Sky;
	}

	return TextureType::Default;
}

fn read_u8(reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> u8
{
	reader.read_exact(&mut buf[0..1]).expect("could not read!");
	return u8::from_le_bytes(buf[0..1].try_into().unwrap());
}

fn read_u16(reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> u16
{
	reader.read_exact(&mut buf[0..2]).expect("could not read!");
	return u16::from_le_bytes(buf[0..2].try_into().unwrap());
}

fn read_u32(reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> u32
{
	reader.read_exact(&mut buf[0..4]).expect("could not read!");
	return u32::from_le_bytes(buf[0..4].try_into().unwrap());
}

fn read_i32(reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> i32
{
	reader.read_exact(&mut buf[0..4]).expect("could not read!");
	return i32::from_le_bytes(buf[0..4].try_into().unwrap());
}

fn read_f32(reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> f32
{
	reader.read_exact(&mut buf[0..4]).expect("could not read!");
	return f32::from_le_bytes(buf[0..4].try_into().unwrap());
}

fn read_vec3(reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> Vector3
{
	return Vector3 { x: read_f32(reader, buf), y: read_f32(reader, buf), z: read_f32(reader, buf) };
}

fn read_string16(reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> String
{
	reader.read_exact(&mut buf[0..16]).expect("could not read!");
	return String::from_utf8(buf[0..16].to_vec()).expect("Not a UTF8 string!");
}

fn read_dir_entry(reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> LumpHeader
{
	return LumpHeader { offset: read_i32(reader, buf), size: read_i32(reader, buf) };
}

pub fn to_bsp(point: Vector3) -> Vector3 {
    return Vector3::new(point.z, point.x, point.y);
}

pub fn to_wld(point: Vector3) -> Vector3 {
    return Vector3::new(point.y, point.z, point.x);
}
