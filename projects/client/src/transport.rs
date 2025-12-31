use shared::Message;
use gns::*;
use std::net::IpAddr;
use std::sync::Arc;

pub struct Transport
{
    client: GnsSocket<IsClient>
}

impl Transport
{
    pub fn new(gns_global: Arc<GnsGlobal>, addr: IpAddr, port: u16) -> Result<Transport, ()> {
        // Create a new [`GnsSocket`], the index type [`IsCreated`] is used to determine the state of the socket.
        // The [`GnsSocket::new`] function is only available for the [`IsCreated`] state. This is the initial state of the socket.
        let gns_socket = GnsSocket::<IsCreated>::new(gns_global.clone());

        // We now do a transition from [`IsCreated`] to the [`IsClient`] state.
        // The [`GnsSocket::connect`] operation does this transition for us.
        // Since we are now using a client socket, we have access to a different set of operations.
        let client = gns_socket.connect(addr, port)?;

        return Ok(Transport { client });
    }

    pub fn poll_messages(&self, mut msg_callback: impl FnMut(Message)) {
        loop {
            let num_msg = self.client.poll_messages::<100>(|message| {
                msg_callback(Message::from_bytes(message.payload()))
            });

            if let Some(n) = num_msg &&
                n < 100 {
                break;
            }
        }

        loop
        {
            // Don't do anything with events.
            // One would check the event for connection status, i.e. doing something when we are connected/disconnected from the server.
            let num_msg = self.client.poll_event::<100>(|ev| {
                let info = ev.info();
                let old_state = ev.old_state();
                let new_state = info.state();
                let end_reason = info.end_reason();
                let end_debug = info.end_debug();
                println!("connection event: {old_state:?} -> {new_state:?}. Reason: {end_reason:?} Debug: {end_debug}");
            });

            if num_msg < 100usize {
                break;
            }
        }
    }
}
