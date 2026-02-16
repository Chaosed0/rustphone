use byteorder::LittleEndian;
use byteorder::ReadBytesExt;
use std::io::BufReader;
use std::io::Read;
use strum_macros::FromRepr;

#[repr(u8)]
#[derive(FromRepr)]
pub enum Message {
    HelloFromClient(u64, String) = 0,
    HelloFromServer(String),
    Chat(String),
    Move { x: u8, y: u8 }
}

impl Message {
    pub fn from_bytes(bytes: &[u8]) -> Option<Message> {
        let mut reader = BufReader::new(bytes);
        let msg_type = reader.read_u8().ok()?;
        let mut msg = Message::from_repr(msg_type)?;

        match &mut msg {
            Message::HelloFromClient(id, s) => {
                *id = reader.read_u64::<LittleEndian>().ok()?;
                reader.read_to_string(s).ok()?;
            }
            Message::HelloFromServer(s) => {
                reader.read_to_string(s).ok()?;
            }
            Message::Chat(s) => {
                reader.read_to_string(s).ok()?;
            }
            Message::Move { x, y } => {
                *x = bytes[1];
                *y = bytes[2];
            }
        }

        return Some(msg);
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut vec : Vec<u8> = Vec::new();
        vec.push(self.discriminant());

        match self {
            Message::HelloFromClient(id, s) => {
                vec.extend_from_slice(&id.to_le_bytes());
                vec.extend_from_slice(s.as_bytes());
            }
            Message::HelloFromServer(s) => {
                vec.extend_from_slice(s.as_bytes());
            }
            Message::Chat(s) => {
                vec.extend_from_slice(s.as_bytes());
            }
            Message::Move { x, y } => {
                vec.push(*x);
                vec.push(*y);
            }
        }

        return vec;
    }

    fn discriminant(&self) -> u8 {
        // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)` `union`
        // between `repr(C)` structs, each of which has the `u8` discriminant as its first
        // field, so we can read the discriminant without offsetting the pointer.
        unsafe { *<*const _>::from(self).cast::<u8>() }
    }
}
