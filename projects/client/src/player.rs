use std::f32::consts::PI;

use enumset::EnumSet;
use raylib::prelude::*;
use crate::{bsp::LeafContentsSet, bsp_query::*};

const MOVE_SPEED: f32 = 256f32;
const JUMP_SPEED: f32 = 300f32;
const GRAVITY: f32 = 550f32;
const DEPEN: f32 = 0.01f32;
const MAX_STEP: f32 = 20f32;

pub struct Player {
    pub movement: Vector3,
    pub yaw: f32,
    pub pitch: f32,
    pub jump: bool,
    pub sprint: bool,
    pub free_move: bool,

    is_grounded: bool,
    y_speed: f32,

    pub pos: Vector3,
}

impl Player {
    pub fn new(pos: Vector3) -> Player {
        return Player {
            movement: Vector3::ZERO,
            yaw: 0f32,
            pitch: 0f32,
            jump: false,
            sprint: false,
            free_move: false,
            is_grounded: true,
            y_speed: 0f32,
            pos: pos
        };
    }

    pub fn forward(&mut self) -> Vector3 {
        let right = Vector3::X.rotate_axis(Vector3::Y, self.yaw);
        return Vector3::Z.rotate_axis(Vector3::Y, self.yaw).rotate_axis(right, self.pitch);
    }

    pub fn update<'a>(&mut self, bsp: &'a impl BspQuery<'a>, dt: f32) {
        let movement = self.movement.try_normalize().unwrap_or(Vector3::ZERO);

        if self.free_move {
            self.free_move_update(dt);
            return;
        }

        // Planar movement
        if movement.x.abs() > 0.01f32 || movement.z.abs() > 0.01f32 {
            let forward = Vector3::Z.rotate_axis(Vector3::Y, self.yaw);
            let right = Vector3::X.rotate_axis(Vector3::Y, self.yaw);
            let delta = MOVE_SPEED * dt;
            let move_dir = forward * movement.z + right * movement.x;

            let intersect = ray_intersect(bsp, self.pos, move_dir, delta, *DPASS);

            let mut new_pos = match intersect {
                Some(ref intersect) => intersect.position + intersect.normal * DEPEN,
                None => self.pos + move_dir * delta
            };

			// If we collided something, do some slide routines
			if let Some(ref intersect) = intersect {
				// Try climbing steps by raycasting upward through solids at the move location
				let passable = DPASS.complement();
				let desired_move = self.pos + move_dir * delta;

				println!("Attempting stair climb: {:?} {:?} {:?} {:?}", self.pos, desired_move, MAX_STEP, passable);
				let climb_intersect = ray_intersect_debug(bsp, desired_move, Vector3::Y, MAX_STEP, passable);

				if let Some(climb_intersect) = climb_intersect {
					// Normal is negated because it points into the solid
					new_pos = climb_intersect.position - intersect.normal * DEPEN;
				} else {
					// If step climbing failed, try sliding along the collision like a wall
					let remainder = (self.pos + move_dir * delta) - new_pos;

					let mut wall_up = intersect.normal.cross(remainder).normalize();
					wall_up.x = 0f32;
					wall_up.z = 0f32;

					if wall_up.length_squared() > 0.01f32 {
						wall_up = wall_up.normalize();
						let wall_along = intersect.normal.normalize().rotate_axis(wall_up, PI * 0.5f32);
						let slide_length = wall_along.dot(remainder);

						//println!("Attempting slide: {:?} {:?} {:?} {:?} {:?}", intersect, remainder, wall_up, wall_along, slide_length);

						if slide_length > 0.01f32 {
							let slide_intersect = ray_intersect(bsp, new_pos, wall_along, slide_length, *DPASS);
							new_pos = match slide_intersect {
								Some(ref intersect) => intersect.position + intersect.normal * DEPEN,
								None => new_pos + wall_along * slide_length
							};
						}
					}
				}
			}

            //println!("MOVE: {:?} {:?} {:?} {:?} {:?} {:?}->{:?}", forward, right, delta, move_dir, intersect, self.pos, new_pos);
            self.pos = new_pos;
        }

        // Grounded check
        if self.is_grounded || self.y_speed <= 0f32 {
            let intersect = ray_intersect(bsp, self.pos, -Vector3::Y, 0.1f32, *DPASS);

			//println!("Grounded Check: {:?} {:?}", self.y_speed, intersect);

            if let Some(intersect) = intersect {
				if !self.is_grounded {
					//println!("BECAME GROUNDED");
					self.is_grounded = true;
					self.y_speed = 0f32;
					self.pos = intersect.position + intersect.normal * DEPEN;
				}
            } else {
				if self.is_grounded {
					//println!("BECAME UNGROUNDED");
					self.is_grounded = false;
				}
			}
        }

        // Y Movement
        if self.y_speed.abs() > 0.01f32 {
            let y_delta = self.y_speed.abs() * dt;
            let dir = Vector3::Y * self.y_speed.signum();
            let intersect = ray_intersect(bsp, self.pos, dir, y_delta, *DPASS);

            let new_pos = match intersect {
                Some(ref intersect) => intersect.position + intersect.normal * DEPEN,
                None => self.pos + dir * y_delta
            };

            //println!("YMOVE: {:?} {:?} {:?} {:?} {:?}->{:?}", self.y_speed, y_delta, dir, intersect, self.pos, new_pos);

            if intersect != None && self.y_speed > 0f32 {
                self.y_speed = 0f32;
            }

            self.pos = new_pos;
        }

		// Gravity
		if !self.is_grounded {
            self.y_speed -= GRAVITY * dt;
		}

        // Jump handling
        if self.jump {
            self.jump = false;

            if self.is_grounded {
                self.is_grounded = false;
                self.y_speed = JUMP_SPEED;
                println!("Begin Jump {:?}", self.y_speed);
            }
        }
    }

    fn free_move_update(&mut self, dt: f32) {
        let movement = self.movement.try_normalize().unwrap_or(Vector3::ZERO);

        let up = Vector3::Y;
        let right = Vector3::X.rotate_axis(up, self.yaw);
        let forward = Vector3::Z.rotate_axis(up, self.yaw).rotate_axis(right, self.pitch);

        let delta = MOVE_SPEED * dt;
        let move_dir = forward * movement.z + right * movement.x + up * movement.y;

        self.pos += move_dir * delta;
    }
}
