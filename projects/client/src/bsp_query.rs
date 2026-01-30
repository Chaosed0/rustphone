use raylib::prelude::*;
use crate::bsp::*;

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

pub fn ray_intersect<'a>(bsp: &'a impl BspQuery<'a>, point: Vector3, dir: Vector3, dist: f32) -> Option<Vector3> {
    let point = to_bsp(point);
    let dir = to_bsp(dir);
    
    if dir.length() < 0.0001f32 { return Some(point); }

    let dir = dir.normalize();
    //println!("Raycast {:?} {:?} {:?}", point, dir, dist);
    let d = ray_intersect_recursive(bsp, point, dir, dist, 0);
    if let Some(d) = d {
        //println!("  Got back {:?}", d);
        return Some(to_wld(point + dir * d));
    } else {
        return None;
    }
}

fn ray_intersect_recursive<'a>(bsp: &'a impl BspQuery<'a>, point: Vector3, dir: Vector3, dist: f32, idx: i32) -> Option<f32> {
    if idx < 0 {
        let contents = bsp.get_contents(idx);
        if contents == LeafContents::Empty {
            return None;
        } else {
            return Some(0f32);
        }
    }

    loop {
        let node = &bsp.get_node(idx as usize);
        let plane = &bsp.get_plane(node.plane_index as usize);
        let d = point.dot(plane.normal) - plane.dist;
        let n = dir.dot(plane.normal);

        let dist_to_plane = (d / n).abs();

		// If the point is on the plane, then we need some special handling
		if d.abs() < 0.01f32 {
			// If the direction is pointing into or away from the plane, then use the corresponding child
			if n > 0.01f32 {
				//println!("  On plane facing away from it, using node {:?}", node.children[0]);
				return ray_intersect_recursive(bsp, point, dir, dist, node.children[0]);
			} else if n < -0.01f32 {
				//println!("  On plane facing into it, using node {:?}", node.children[0]);
				return ray_intersect_recursive(bsp, point, dir, dist, node.children[1]);
			} else {
				//println!("  On plane parallel to it");

				// If the vector is parallel to the plane, then check both sides with a small offset.
				let d1 = ray_intersect_recursive(bsp, point + plane.normal * 0.01f32, dir, dist, node.children[0]);
				let d2 = ray_intersect_recursive(bsp, point - plane.normal * 0.01f32, dir, dist, node.children[1]);

				// If we immediately encountered a solid, then ignore it and use the other side's result.
				// Otherwise, use the closest intersection.
				if let Some(d1) = d1 {
					if d1 == 0f32 {
						//println!("  Intersected child 0, using child 1");
						return d2;
					} else if let Some(d2) = d2 {
						//println!("  Using min of child 0 and 1");
						return Some(d1.min(d2));
					} else {
						//println!("  Using child 0");
						return Some(d1);
					}
				} else if let Some(d2) = d2 {
					if d2 == 0f32 {
						//println!("  Intersected child 1, using child 0");
						return d1;
					} else {
						//println!("  Using child 1");
						return Some(d2);
					}
				}
			}
		}

        let close_child = if d > 0f32 { node.children[0] } else { node.children[1] };
        let far_child = if d > 0f32 { node.children[1] } else { node.children[0] };

        // Point and endpoint are both on the same side of the plane, i.e.
        //  - Direction is facing away from the plane, or
        //  - Direction is facing towards the plane and dist won't reach the plane
        if d.signum() == n.signum() || dist_to_plane > dist {
            //println!("  Same side. {:?} {:?} {:?} {:?} {:?}", d, n, dist_to_plane, dist, close_child);
            return ray_intersect_recursive(bsp, point, dir, dist, close_child);
        }

        // We will intersect this plane, so divide the ray in 2
        // Raycast the closer side
        let d1 = ray_intersect_recursive(bsp, point, dir, dist_to_plane - 0.01f32, close_child);
        //println!("  First side {:?} {:?} {:?}", dist_to_plane, node.children[0], d1);

        // If it hit something, immediately use that
        if let Some(d1) = d1 {
            return Some(d1);
        }

        // Raycast the far side
        let midpoint = point + dir * (dist_to_plane + 0.01f32);

        let d2 = ray_intersect_recursive(bsp, midpoint, dir, dist - dist_to_plane, far_child);
        //println!("  Second side {:?} {:?} {:?} {:?}", midpoint, dist - dist_to_plane, far_child, d2);

        // If it hit, use that, or return no contact
        if let Some(d2) = d2 {
            return Some(dist_to_plane + d2);
        } else {
            return None;
        }
    }
}

/*
pub fn get_leafs_containing_sphere(bsp: &Bsp, point: Vector3, radius: f32) {
}
*/
