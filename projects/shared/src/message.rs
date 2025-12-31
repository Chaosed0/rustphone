
pub enum Message {
    Chat(String),
    Move { x: u8, y: u8 }
}

impl Message {
    pub fn from_bytes(bytes: &[u8]) -> Message {
        let msg_type = bytes[0];

        match msg_type {
            0 => {
                let s = String::from_utf8(bytes[1..].to_vec()).expect("UHH");
                return Message::Chat(s);
            }
            1 => {
                return Message::Move { x: bytes[1], y: bytes[2] };
            }
            _ => panic!("no message type for {msg_type}")
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut vec : Vec<u8> = Vec::new();

        match self {
            Message::Chat(s) => {
                vec.push(0);
                vec.extend_from_slice(s.as_bytes())
            }
            Message::Move { x, y } => {
                vec.push(1);
                vec.push(*x);
                vec.push(*y);
            }
        }

        return vec;
    }
}
