use byteorder::{ByteOrder, LittleEndian};
use raylib::core::math::*;
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;
use std::io::BufReader;

const BSP2_VER: i32 = (('B' as i32) << 0) | (('S' as i32) << 8) | (('P' as i32) << 16) | (('2' as i32) << 24);

#[derive(Default)]
struct DirEntry
{
	offset: i32,
	size: i32
}

#[derive(Default)]
struct BspHeader
{
	version: i32,
	entities: DirEntry,
	planes: DirEntry,
	mip_tex: DirEntry,
	vertices: DirEntry,
	visilist: DirEntry,
	nodes: DirEntry,
	faces: DirEntry,
	lightmaps: DirEntry,
	clipnodes: DirEntry,
	leaves: DirEntry,
	face_list: DirEntry,
	face_edges: DirEntry,
	edge_list: DirEntry,
	models: DirEntry,
}

struct Model
{
	bound: BoundingBox,
	origin: Vector3,
	bsp_node: i64,
	clip_node_first: i64,
	clip_node_second: i64,
	reserved: i64,
	leafs_num: i64,
	face_id: i64,
	faces_num: i64
}

struct Edge
{
	v0: u16,
	v1: u16
}

struct Surface
{
	vec_s: Vector3,
	dist_s: f32,
	vec_t: Vector3,
	dist_t: f32,
	tex_id: u64,
	animated: u64
}

struct Face
{
	plane_id: u16,
	side: u16,
	edge_id: i64,
	edge_num: u16,
	texinfo_id: u16,
	light_type: u8,
	light_base: u8,
	light0: u8,
	light1: u8,
	light_map: i64
}

struct MipHeader
{
	tex_num: i64,
	offset: [i64],
}

struct MipTex
{
	name: String,
	width: u64,
	height: u64,
	offset1: u64,
	offset2: u64,
	offset4: u64,
	offset8: u64
}

struct Node
{
	plane_id: i64,
	front: u16,
	back: u16,
	bounds: BoundingBox,
	face_id: u16,
	face_num: u16
}

struct Leaf
{
	leaf_type: i64,
	vislist: i64,
	bounds: BoundingBox,
	first_face_id: u16,
	first_face_num: u16,
	snd_water: u8,
	snd_sky: u8,
	snd_slime: u8,
	snd_lava: u8
}


pub fn load_bsp(filename: &str)
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
	header.faces = read_dir_entry(&mut reader, &mut buf);
	header.lightmaps = read_dir_entry(&mut reader, &mut buf);
	header.clipnodes = read_dir_entry(&mut reader, &mut buf);
	header.leaves = read_dir_entry(&mut reader, &mut buf);
	header.face_list = read_dir_entry(&mut reader, &mut buf);
	header.face_edges = read_dir_entry(&mut reader, &mut buf);
	header.edge_list = read_dir_entry(&mut reader, &mut buf);
	header.models = read_dir_entry(&mut reader, &mut buf);

	println!("Loading bsp {:?}", filename);
	println!("  version: {:?} ({:?})", header.version, BSP2_VER);
	println!("  entities: {:?} {:?}", header.entities.offset, header.entities.size);
	println!("  planes: {:?} {:?}", header.planes.offset, header.planes.size);
	println!("  mip_tex: {:?} {:?}", header.mip_tex.offset, header.mip_tex.size);
	println!("  vertices: {:?} {:?}", header.vertices.offset, header.vertices.size);
	println!("  visilist: {:?} {:?}", header.visilist.offset, header.visilist.size);
	println!("  nodes: {:?} {:?}", header.nodes.offset, header.nodes.size);
	println!("  faces: {:?} {:?}", header.faces.offset, header.faces.size);
	println!("  lightmaps: {:?} {:?}", header.lightmaps.offset, header.lightmaps.size);
	println!("  clipnodes: {:?} {:?}", header.clipnodes.offset, header.clipnodes.size);
	println!("  leaves: {:?} {:?}", header.leaves.offset, header.leaves.size);
	println!("  face_list: {:?} {:?}", header.face_list.offset, header.face_list.size);
	println!("  face_edges: {:?} {:?}", header.face_edges.offset, header.face_edges.size);
	println!("  edge_list: {:?} {:?}", header.edge_list.offset, header.edge_list.size);
	println!("  models: {:?} {:?}", header.models.offset, header.models.size);
}

fn read_i32(reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> i32
{
	reader.read_exact(&mut buf[0..4]).expect("could not read!");
	return LittleEndian::read_i32(&buf[0..4]);
}

fn read_dir_entry(reader: &mut BufReader<File>, buf: &mut Vec<u8>) -> DirEntry
{
	return DirEntry { offset: read_i32(reader, buf), size: read_i32(reader, buf) };
}