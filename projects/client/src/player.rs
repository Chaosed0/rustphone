use raylib::prelude::*;
use crate::bsp_query;
use crate::bsp_query::BspQuery;

const MOVE_SPEED: f32 = 256f32;
const JUMP_SPEED: f32 = 300f32;
const GRAVITY: f32 = 550f32;
const DEPEN: f32 = 0.01f32;

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

            let intersect = bsp_query::ray_intersect(bsp, self.pos, move_dir, delta);

            let new_pos = match intersect {
                Some(intersect) => intersect - move_dir * DEPEN,
                None => self.pos + move_dir * delta
            };

            //println!("MOVE: {:?} {:?} {:?} {:?} {:?} {:?}->{:?}", forward, right, delta, move_dir, intersect, self.pos, new_pos);
            self.pos = new_pos;
        }

        // Grounded check
        if self.is_grounded || self.y_speed <= 0f32 {
            let intersect = bsp_query::ray_intersect(bsp, self.pos, -Vector3::Y, 0.1f32);

			//println!("Grounded Check: {:?} {:?}", self.y_speed, intersect);

            if let Some(intersect) = intersect {
				if !self.is_grounded {
					//println!("BECAME GROUNDED");
					self.is_grounded = true;
					self.y_speed = 0f32;
					self.pos = intersect + Vector3::Y * DEPEN;
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
            let intersect = bsp_query::ray_intersect(bsp, self.pos, dir, y_delta);

            let new_pos = match intersect {
                Some(intersect) => intersect - dir * DEPEN,
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
