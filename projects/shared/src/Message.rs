
struct Message {
    Chat(String),
    Move { x: u8, y: u8 }
}

impl Message {
    fn from_bytes(bytes: &[u8]) -> Message {
        let msg_type = bytes[0];

        match msg_type {
            0 => {
                let s = String::from_utf8(bytes[1..]);
                return Message::Chat { s };
            }
            1 => {
                return Message::Move { bytes[1], bytes[2] };
            }
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let vec : Vec<u8> = Vec::new();

        match self {
            Message::Chat(s) => {
                vec.push(0);
                vec.extend_from_slice(s.as_bytes())
            }
            Message::Move(x,y) => {
                vec.push(1);
                vec.push(x);
                vec.push(y);
            }
        }

        return vec;
    }
}
