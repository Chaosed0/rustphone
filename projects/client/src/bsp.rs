use raylib::core::math::*;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

const BSP2_VER: i32 = (('B' as i32) << 0) | (('S' as i32) << 8) | (('P' as i32) << 16) | (('2' as i32) << 24);
const MAX_LIGHTMAPS: usize = 4;

pub struct Bsp
{
	pub textures: Vec<MipTex>,
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

struct Model
{
	name: String,
	path_id: u32,
	model_type: ModelType,
	num_frames: i32,

	flags: i32,
	sort_key: u32,

	mins: Vector3,
	maxs: Vector3,
	ymins: Vector3,
	ymaxs: Vector3,
	rmins: Vector3,
	rmaxs: Vector3,

	clip_box: bool,
	clip_mins: Vector3,
	clip_maxs: Vector3,

	// BRUSH

	first_model_surf: i32,
	num_model_surf: i32,

	submodels: Vec<Model>,
	planes: Vec<Plane>,
	leaf: Vec<Leaf>,
	verts: Vec<Vector3>,
	edges: Vec<Edge>,
	nodes: Vec<Node>,
	tex_infos: Vec<TexInfo>,
	surfs: Vec<Surface>,
	surf_edges: Vec<i32>,
	clip_nodes: Vec<ClipNode>,
	mark_surfs: Vec<i32>,
	hulls: Vec<Hull>,

	first_cmd: i32,
	tex_ofs: Vec<i32>,
	used_textures: Vec<i32>,

	vis_data: Vec<u8>,
	light_data: Vec<u8>,
	entities: Vec<i8>,

	lit_file: bool,
	vis_warn: bool,

	bsp_version: i32,
	has_lit_water: i32,

	// ALIAS

	mesh_vbo: u32,
	mesh_indexes_vbo: u32,

	// ADDITIONAL

	//cache: CacheUser
}

enum TextureType
{
	Default,
	Cutout,
	Sky,
	Lava,
	Slime,
	Tele,
	Water
}

struct Plane
{
	normal: Vector3,
	dist: f32,
	p_type: u8,
	sign: u8,

	_pad0: u8,
	_pad1: u8
}

struct Leaf
{
	contents: i32,
	vis_frame: i32,
	mins: Vector3,
	maxs: Vector3,

	parent_node_index: u32,

	compressed_vis: Vec<u8>,
	first_mark_surface: Vec<i32>,
	num_mark_surfaces: i32,
	key: i32,
	ambient_sound_level: [u8;4]
}

struct Node
{
	contents: i32,
	vis_frame: i32,
	mins: Vector3,
	maxs: Vector3,

	parent_node_index: u32,

	plane_index: u32,
	children: [Option<Box<Node>>;2],

	first_surf: u32,
	num_surf: u32
}

struct TexInfo
{
	vec1: Vector4,
	vec2: Vector4,
	tex_num: i32,
	flags: i32
}

struct Edge
{
	v0: u16,
	v1: u16
}

struct Surface
{
	plane: u32,
	mins: Vector3,
	maxs: Vector3,
	flags: i32,
	
	vbo_firstvert: i32, // Index of first vertex in VBO
	first_edge: i32, // Lookup in model->surfedges, negative are backwards
	num_edges: i16,

	lightmap_tex_num: i16,
	extent_x: i16,
	extent_y: i16,
	light_s: i16,
	light_t: i16,

	styles: [u8;MAX_LIGHTMAPS],
	samples: Vec<u8>, // size: numstyles * surfsize

	texture_min_x: i32,
	texture_min_y: i32,
	tex_info: u32
}

struct ClipNode
{
	plane_num: i32,
	children: [i32;2] // negatives are contents
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

pub struct MipTex
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
	header.tex_info = read_dir_entry(&mut reader, &mut buf);
	header.nodes = read_dir_entry(&mut reader, &mut buf);
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

	read_verts(header.vertices, &mut reader, &mut buf);
	read_edges(header.edges, &mut reader, &mut buf);
	read_surf_edges(header.surf_edges, &mut reader, &mut buf);
	let mips = read_mips(header.mip_tex, &mut reader, &mut buf);

	return Bsp { textures: mips };
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
		edges.push(Edge { v0: read_u16(reader, buf), v1: read_u16(reader, buf) });
	}

	return edges;
}

fn read_surf_edges(header: LumpHeader, reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> Vec<i32>
{
	reader.seek(std::io::SeekFrom::Start(header.offset as u64))
		.unwrap_or_else(|err| panic!("{err}: Invalid surf_edge offset {:?}", header.offset));

	let mut surf_edges = Vec::<i32>::new();
	let count = header.size as usize / size_of::<i32>();

	for _ in 0..count
	{
		surf_edges.push(read_i32(reader, buf));
	}

	return surf_edges;
}

fn read_mips(header: LumpHeader, reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> Vec<MipTex>
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

	for i in 0..num_tex
	{
		let data_offset = read_i32(reader, buf);
		dataofs.push(data_offset);
	}

	let mut mip_texs = Vec::<MipTex>::new();

	for data_offset in dataofs
	{
		if data_offset < 0 {
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

		mip_texs.push(MipTex { name, width, height, offset1, offset2, offset4, offset8, tex_type, pixels });
	}

	return mip_texs;
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

fn read_i64(reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> i64
{
	reader.read_exact(&mut buf[0..8]).expect("could not read!");
	return i64::from_le_bytes(buf[0..8].try_into().unwrap());
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