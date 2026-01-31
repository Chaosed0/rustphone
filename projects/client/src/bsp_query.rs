use raylib::prelude::*;
use enumset::*;
use crate::bsp::*;
use lazy_static::lazy_static;

lazy_static! {
	pub static ref DPASS: EnumSet<LeafContentsSet> = EnumSet::from(LeafContentsSet::Empty);
}

#[derive(PartialEq, Debug)]
pub struct Intersection {
	pub position: Vector3,
	pub normal: Vector3
}

#[derive(PartialEq, Debug)]
struct IntersectionInternal {
	d: f32,
	data: Option<IntersectData>,
	contents: LeafContents,
}

#[derive(PartialEq, Debug)]
struct IntersectData {
	plane_index: usize,
	reverse_normal: bool
}

pub struct BspQueryNode {
    plane_index: usize,
    children: [i32;2]
}

pub trait BspQuery<'a> {
    fn get_node(&self, idx: usize) -> BspQueryNode;
    fn get_plane(&self, idx: usize) -> &Plane;
    fn get_contents(&self, idx: i32) -> LeafContents;
}

pub struct BspVisQuery<'a> {
    bsp: &'a Bsp
}

impl<'a> BspVisQuery<'a> {
    pub fn new(bsp: &'a Bsp) -> BspVisQuery<'a> {
        return BspVisQuery { bsp };
    }
}


impl<'a> BspQuery<'a> for BspVisQuery<'a> {
    fn get_node(&self, idx: usize) -> BspQueryNode {
        let node = &self.bsp.nodes[idx];
        return BspQueryNode { plane_index: node.plane_index as usize, children: node.children };
    }

    fn get_plane(&self, idx: usize) -> &'a Plane {
        return &self.bsp.planes[idx];
    }

    fn get_contents(&self, idx: i32) -> LeafContents {
        return self.bsp.leafs[-(idx+1) as usize].contents;
    }
}

pub struct BspClipQuery<'a> {
    bsp: &'a Bsp
}

impl<'a> BspClipQuery<'a> {
    pub fn new(bsp: &'a Bsp) -> BspClipQuery<'a> {
        return BspClipQuery { bsp };
    }
}


impl<'a> BspQuery<'a> for BspClipQuery<'a> {
    fn get_node(&self, idx: usize) -> BspQueryNode {
        let node = &self.bsp.clip_nodes[idx];
        return BspQueryNode { plane_index: node.plane_index as usize, children: node.children };
    }

    fn get_plane(&self, idx: usize) -> &'a Plane {
        return &self.bsp.planes[idx];
    }

    fn get_contents(&self, idx: i32) -> LeafContents {
        return LeafContents::from_repr(idx).unwrap();
    }
}

pub fn point_intersect<'a>(bsp: &'a impl BspQuery<'a>, point: Vector3) -> LeafContents {
    let point = to_bsp(point);
    let mut idx = 0;
    loop {
        let node = &bsp.get_node(idx);
        let plane = &bsp.get_plane(node.plane_index);
        let d = point.dot(plane.normal) - plane.dist;

        let next_idx = if d > 0f32 {
            node.children[0]
        } else {
            node.children[1]
        };

        //println!("  iter {:?}: {:?} -- {:?} {:?} {:?} {:?}", i, next_idx, idx, d, plane.normal, plane.dist);

        if next_idx < 0 {
            //println!("  got leaf at {:?}", -(next_idx + 1));
            return bsp.get_contents(next_idx as i32);
        } else {
            idx = next_idx as usize;
        }
    }
}

pub fn ray_intersect_debug<'a>(bsp: &'a impl BspQuery<'a>, point: Vector3, dir: Vector3, dist: f32, passable: EnumSet<LeafContentsSet>) -> Option<Intersection> {
	ray_intersect_internal(bsp, point, dir, dist, passable, true)
}

pub fn ray_intersect<'a>(bsp: &'a impl BspQuery<'a>, point: Vector3, dir: Vector3, dist: f32, passable: EnumSet<LeafContentsSet>) -> Option<Intersection> {
	ray_intersect_internal(bsp, point, dir, dist, passable, false)
}

fn ray_intersect_internal<'a>(bsp: &'a impl BspQuery<'a>, point: Vector3, dir: Vector3, dist: f32, passable: EnumSet<LeafContentsSet>, debug: bool) -> Option<Intersection> {
    let point = to_bsp(point);
    let dir = to_bsp(dir);
    
    if dir.length() < 0.0001f32 {
		return Some(Intersection { position: point, normal: Vector3::ZERO });
	}

    let dir = dir.normalize();
    if debug { println!("Raycast {:?} {:?} {:?}", point, dir, dist); }
    let d = ray_intersect_recursive(bsp, point, dir, dist, passable, 0, None, debug);
    if let Some(d) = d {
		let normal = d.data.as_ref().map(|data| bsp.get_plane(data.plane_index).normal * if data.reverse_normal { -1f32 } else { 1f32 });
		let normal = normal.unwrap_or(Vector3::ZERO);

        if debug { println!("  Got back {:?}, normal: {:?}", d, normal); }

        return Some(Intersection { position: to_wld(point + dir * d.d), normal: to_wld(normal) });
    } else {
        if debug { println!("  No intersection"); }
        return None;
    }
}

fn ray_intersect_recursive<'a>(bsp: &'a impl BspQuery<'a>, point: Vector3, dir: Vector3, dist: f32, passable: EnumSet<LeafContentsSet>, idx: i32, data: Option<IntersectData>, debug: bool) -> Option<IntersectionInternal> {
    if idx < 0 {
        let contents = bsp.get_contents(idx);
		let set_value = LeafContentsSet::from_repr((-idx - 1) as usize).unwrap();

		if debug { println!("  Intersected: {idx} {contents:?} {passable:?} {:?}.", passable.contains(set_value)); }

        if passable.contains(set_value) {
            return None;
        } else {
            return Some(IntersectionInternal { d: 0f32, data: data, contents: contents });
        }
    }

    loop {
		let idx = idx as usize;
        let node = &bsp.get_node(idx);
		let plane_index = node.plane_index;
        let plane = bsp.get_plane(plane_index);
        let d = point.dot(plane.normal) - plane.dist;
        let n = dir.dot(plane.normal);

        let dist_to_plane = (d / n).abs();

		// If the point is on the plane, then we need some special handling.
		if d.abs() < 0.01f32 {
			if debug { println!("  On plane. {idx} {plane_index} {d} {n} {:?} {:?}", plane.normal, plane.dist) }
			if n > 0.01f32 {
				if debug { println!("  Facing away from it, using node {:?}", node.children[0]); }
				return ray_intersect_recursive(bsp, point, dir, dist, passable, node.children[0], Some(IntersectData { plane_index, reverse_normal: true }), debug);
			} else if n < -0.01f32 {
				if debug { println!("  Facing into it, using node {:?}", node.children[1]); }
				return ray_intersect_recursive(bsp, point, dir, dist, passable, node.children[1], Some(IntersectData { plane_index, reverse_normal: false }), debug);
			} else {
				if debug { println!("  Parallel! First side:"); }

				let d1 = ray_intersect_recursive(bsp, point + plane.normal * 0.01f32, dir, dist, passable, node.children[0], Some(IntersectData { plane_index, reverse_normal: true }), debug);

				if debug { println!("  Parallel! Second side:"); }
				let d2 = ray_intersect_recursive(bsp, point - plane.normal * 0.01f32, dir, dist, passable, node.children[1], Some(IntersectData { plane_index, reverse_normal: false }), debug);

				// If we immediately encountered a solid, then ignore it and use the other side's result.
				// Otherwise, use the closest intersection.
				if let Some(d1) = d1 {
					if d1.d == 0f32 {
						if debug { println!("  Intersected child 0, using child 1"); }
						return d2;
					} else if let Some(d2) = d2 {
						if debug { println!("  Using min of child 0 and 1"); }
						if (d1.d < d2.d) {
							return Some(d1);
						} else {
							return Some(d2);
						}
					} else {
						if debug { println!("  Using child 0"); }
						return Some(d1);
					}
				} else if let Some(d2) = d2 {
					if d2.d == 0f32 {
						if debug { println!("  Intersected child 1, using child 0"); }
						return d1;
					} else {
						if debug { println!("  Using child 1"); }
						return Some(d2);
					}
				}
			}
		}

		let far_reverse_normal = d <= 0f32;
        let close_child = if d > 0f32 { node.children[0] } else { node.children[1] };
        let far_child = if d > 0f32 { node.children[1] } else { node.children[0] };

        // Point and endpoint are both on the same side of the plane, i.e.
        //  - Direction is facing away from the plane, or
        //  - Direction is facing towards the plane and dist won't reach the plane
        if d.signum() == n.signum() || dist_to_plane > dist {
            if debug { println!("  Same side. {:?} {:?} {:?} {:?} {:?}", d, n, dist_to_plane, dist, close_child); }
            return ray_intersect_recursive(bsp, point, dir, dist, passable, close_child, data, debug);
        }

        // We will intersect this plane, so divide the ray in 2
        // Raycast the closer side
        if debug { println!("  Split ray {plane_index} {:?} {:?} {d} {n} {dist_to_plane}", plane.normal, plane.dist); }
        let d1 = ray_intersect_recursive(bsp, point, dir, dist_to_plane, passable, close_child, data, debug);
        if debug { println!("  First side {:?} {:?} {:?}", dist_to_plane, close_child, d1); }

        // If it hit something, immediately use that
        if let Some(d1) = d1 {
            return Some(d1);
        }

        // Raycast the far side
        let midpoint = point + dir * dist_to_plane;

        let d2 = ray_intersect_recursive(bsp, midpoint, dir, dist - dist_to_plane, passable, far_child, Some(IntersectData { plane_index, reverse_normal: far_reverse_normal }), debug);
        if debug { println!("  Second side {:?} {:?} {:?} {:?}", midpoint, dist - dist_to_plane, far_child, d2); }

        // If it hit, use that, or return no contact
        if let Some(d2) = d2 {
            return Some(IntersectionInternal { d: dist_to_plane + d2.d, data: d2.data, contents: d2.contents });
        } else {
            return None;
        }
    }
}

/*
pub fn get_leafs_containing_sphere(bsp: &Bsp, point: Vector3, radius: f32) {
}
*/
