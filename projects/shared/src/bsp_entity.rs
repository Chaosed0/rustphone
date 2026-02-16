use crate::bsp::*;
use raylib::prelude::*;
use std::str::FromStr;
use lazy_static::lazy_static;

lazy_static! {
    static ref CLASSNAME_STR: String = "classname".to_string();
}

pub fn of_type<'a>(bsp: &'a Bsp, name: &str) -> impl Iterator<Item = &'a Entity> {
    return bsp.entities.iter().filter(move |entity| {
        if let Some(value) = entity.get(&CLASSNAME_STR) && value == name { true } else { false }
    });
}

impl Entity {
    pub fn get(&self, key: &String) -> Option<&String> {
        return self.map.get(key);
    }

    pub fn get_vec3(&self, key: &String) -> Vector3 {
        let value = self.get(key).unwrap();
        let mut split = value.split_whitespace();

        let xs = split.next();
        let ys = split.next();
        let zs = split.next();

        if xs == None || ys == None || zs == None {
            panic!("Invalid vec3 format for key {:?} in entity: {:?}", key, value);
        }

        let x = f32::from_str(xs.unwrap()).unwrap();
        let y = f32::from_str(ys.unwrap()).unwrap();
        let z = f32::from_str(zs.unwrap()).unwrap();

        return to_wld(Vector3::new(x, y, z));
    }
}
