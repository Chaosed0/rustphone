use shared::Message;
use gns::*;
use crate::transport::sys::ESteamNetworkingConnectionState;
use std::net::IpAddr;
use std::sync::Arc;
use std::collections::HashMap;

pub struct Transport
{
    server: GnsSocket<IsServer>,
    connected_clients: HashMap::<GnsConnection, String>,
    nonce: u16,
}

impl Transport
{
    pub fn new(gns_global: Arc<GnsGlobal>, addr: IpAddr, port: u16) -> Result<Transport, ()> {
        let gns_socket = GnsSocket::<IsCreated>::new(gns_global.clone());
        let server = gns_socket.listen(addr, port)?;
        let connected_clients = HashMap::<GnsConnection, String>::new();

        return Ok(Transport { server, connected_clients, nonce: 0 });
    }

    pub fn poll_messages(&mut self, mut msg_callback: impl FnMut(Message)) {
        loop {
            let num_msg = self.server.poll_messages::<100>(|message| {
                msg_callback(Message::from_bytes(message.payload()))
            });

            if let Some(n) = num_msg &&
                n < 100 {
                break;
            }
        }

        loop
        {
            let num_msg = self.server.poll_event::<100>(|event| {
                match (event.old_state(), event.info().state()) {
                    // A client is about to connect, accept it.
                    (
                        ESteamNetworkingConnectionState::k_ESteamNetworkingConnectionState_None,
                        ESteamNetworkingConnectionState::k_ESteamNetworkingConnectionState_Connecting,
                    ) => {
                        let result = self.server.accept(event.connection());
                        println!("GnsSocket<Server>: accepted new client: {:#?}.", result);
                        if result.is_ok() {
                            self.connected_clients.insert(event.connection(), self.nonce.to_string());
                            /*
                            broadcast_chat(
                                self.connected_clients.keys().copied().collect(),
                                "Server",
                                &format!("A new user joined us, welcome {}", self.nonce),
                            );
                            */
                            self.nonce += 1;
                        }
                        println!("GnsSocket<Server>: number of clients: {:#?}.", self.connected_clients.len());
                    }

                    // A client is connected, we previously accepted it and don't do anything here.
                    // In a more sophisticated scenario we could initial sending some messages.
                    (
                        ESteamNetworkingConnectionState::k_ESteamNetworkingConnectionState_Connecting,
                        ESteamNetworkingConnectionState::k_ESteamNetworkingConnectionState_Connected,
                    ) => {
                    }

                    (_, ESteamNetworkingConnectionState::k_ESteamNetworkingConnectionState_ClosedByPeer |
                        ESteamNetworkingConnectionState::k_ESteamNetworkingConnectionState_ProblemDetectedLocally) => {
                        // Remove the client from the list and close the connection.
                        let conn = event.connection();
                        println!("GnsSocket<Server>: {:#?} disconnected", conn);
                        /*
                        let nickname = &self.connected_clients[&conn];
                        broadcast_chat(
                            self.connected_clients.keys().copied().collect(),
                            "Server",
                            &format!("[{}] lost faith.", nickname),
                        );
                        */
                        self.connected_clients.remove(&conn);
                        // Make sure we cleanup the connection, mandatory as per GNS doc.
                        self.server.close_connection(conn, 0, "", false);
                    }

                    // A client state is changing, perhaps disconnecting
                    // If a client disconnected and it's connection get cleaned up, its state goes back to `ESteamNetworkingConnectionState::k_ESteamNetworkingConnectionState_None`
                    (previous, current) => {
                        println!("GnsSocket<Server>: {:#?} => {:#?}.", previous, current);
                    }
                }
            });

            if num_msg < 100usize {
                break;
            }
        }
    }
}
