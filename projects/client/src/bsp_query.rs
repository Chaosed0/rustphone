use raylib::prelude::*;
use crate::bsp::*;

pub fn wld2bsp(point: Vector3) -> Vector3 {
    return Vector3::new(point.z, point.x, point.y);
}

pub fn bsp2wld(point: Vector3) -> Vector3 {
    return Vector3::new(point.y, point.z, point.x);
}

pub fn get_leaf_containing_point(bsp: &Bsp, point: Vector3) -> &Leaf {
    let point = wld2bsp(point);
    let mut idx = 0;
    let mut i = 0;
    loop {
        let node = &bsp.nodes[idx];
        let plane = &bsp.planes[node.plane_index as usize];
        let d = point.dot(plane.normal) - plane.dist;

        let next_idx = if d > 0f32 {
            node.children[0]
        } else {
            node.children[1]
        };

        //println!("  iter {:?}: {:?} -- {:?} {:?} {:?} {:?}", i, next_idx, idx, d, plane.normal, plane.dist);

        if next_idx < 0 {
            //println!("  got leaf at {:?}", -(next_idx + 1));
            return &bsp.leafs[-(next_idx + 1) as usize];
        } else {
            idx = next_idx as usize;
        }

        i += 1;
    }
}

pub fn point_intersect(bsp: &Bsp, point: Vector3) -> LeafContents {
    let point = wld2bsp(point);
    let mut idx = 0;
    loop {
        let node = &bsp.clip_nodes[idx];
        let plane = &bsp.planes[node.plane_index as usize];
        let d = point.dot(plane.normal) - plane.dist;

        let next_idx = if d > 0f32 {
            node.children[0]
        } else {
            node.children[1]
        };

        if next_idx < 0 {
            return LeafContents::from_repr(next_idx).unwrap();
        } else {
            idx = next_idx as usize;
        }
    }
}

pub fn ray_intersect(bsp: &Bsp, point: Vector3, dir: Vector3, dist: f32) -> Option<Vector3> {
    let point = wld2bsp(point);
    let dir = wld2bsp(dir).normalize();
    //println!("Raycast {:?} {:?} {:?}", point, dir, dist);
    let d = ray_intersect_recursive(bsp, point, dir, dist, 0);
    if let Some(d) = d {
        //println!("  Got back {:?}", d);
        return Some(bsp2wld(point + dir * d));
    } else {
        return None;
    }
}

fn ray_intersect_recursive(bsp: &Bsp, point: Vector3, dir: Vector3, dist: f32, idx: i32) -> Option<f32> {
    if idx < 0 {
        let contents = LeafContents::from_repr(idx).unwrap();
        if contents == LeafContents::Empty {
            return None;
        } else {
            return Some(0f32);
        }
    }

    loop {
        let node = &bsp.clip_nodes[idx as usize];
        let plane = &bsp.planes[node.plane_index as usize];
        let d = point.dot(plane.normal) - plane.dist;
        let n = dir.dot(plane.normal);

        let dist_to_plane = (d / n).abs();

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
