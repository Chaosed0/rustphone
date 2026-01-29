use raylib::prelude::*;
use crate::bsp::*;

const CONTENTS_EMPTY: i32 = -1;
const CONTENTS_SOLID: i32 = -2;
const CONTENTS_WATER: i32 = -3;
const CONTENTS_SLIME: i32 = -4;
const CONTENTS_LAVA: i32 = -5;
const CONTENTS_SKY: i32 = -6;
const CONTENTS_ORIGIN: i32 = -7;
const CONTENTS_CLIP: i32 = -8;

pub fn get_leaf_containing_point(bsp: &Bsp, point: Vector3) -> &Leaf {
    let mut node = &bsp.nodes[0];
    loop {
        let plane = &bsp.planes[node.plane_index as usize];
        let d = point.dot(plane.normal) - plane.dist;

        let next_idx = if d > 0f32 {
            node.children[0]
        } else {
            node.children[1]
        };

        if next_idx < 0 {
            return &bsp.leafs[(-(next_idx)+1) as usize];
        } else {
            node = &bsp.nodes[next_idx as usize];
        }
    }
}
