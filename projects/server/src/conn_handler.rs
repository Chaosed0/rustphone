use shared::message::Message;
use crate::err::Error;

pub struct ConnHandler {
    connections: Vec<Connection>,
}

struct Connection {
    id: u64,
    name: String
}

impl ConnHandler {
    pub fn new() -> ConnHandler {
        return ConnHandler { connections: vec!() }
    }

    pub fn on_connected(&mut self, msg: Message) -> Result<u64, Error> {
        let Message::HelloFromClient(id, s) = msg else {
            return Err(Error::InternalError);
        };

        self.connections.push(Connection { id, name: s });
        return Ok(id);
    }

    pub fn on_disconnected(&mut self, id: u64) -> Result<(), Error> {
        let pos = self.connections.iter().position(|c| c.id == id).ok_or(Error::InternalError)?;
        self.connections.remove(pos);
        return Ok(());
    }
}
