use gns::{GnsGlobal, GnsSocket, IsCreated};
use std::net::Ipv6Addr;
use std::time::Duration;

// Initial the global networking state. Note that this instance must be unique per-process.
static GnsGlobal gns_global = GnsGlobal::get().unwrap();

struct Transport
{
    gns_socket: GnsSocket,
}

impl Transport
{
    fn new() -> Transport {
        // Create a new [`GnsSocket`], the index type [`IsCreated`] is used to determine the state of the socket.
        // The [`GnsSocket::new`] function is only available for the [`IsCreated`] state. This is the initial state of the socket.
        let gns_socket = GnsSocket::<IsCreated>::new(gns_global.clone());

        return Transport { gns_socket, port };
    }

    fn connect(&self, addr: IpAddr, port: u16) -> Result<()>
        // We now do a transition from [`IsCreated`] to the [`IsClient`] state.
        // The [`GnsSocket::connect`] operation does this transition for us.
        // Since we are now using a client socket, we have access to a different set of operations.
        self.gns_socket = self.gns_socket.connect(addr, self.port)?;
    }

    fn poll_messages(&self, msg_callback: FnMut(Message)) {
        loop {
            let num_msg = self.gns_socket.poll_messages::<100>(|message| {
                msg_callback(Message::from_bytes(message.payload))
            });

            if (num_msg < 100) break;
        }

        loop
        {
            // Don't do anything with events.
            // One would check the event for connection status, i.e. doing something when we are connected/disconnected from the server.
            let num_msg = self.gns_socket.poll_event::<100>(|_| { });
            if (num_msg < 100) break;
        }
    }
}
