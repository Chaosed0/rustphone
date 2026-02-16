mod transport;
use transport::Transport;

mod conn_handler;
use conn_handler::ConnHandler;

mod err;

use gns::GnsGlobal;
use std::net::Ipv4Addr;
use std::time::Instant;
use std::time::Duration;

use shared::message::Message;
use shared::bsp::*;

fn main() {
    // Initial the global networking state. Note that this instance must be unique per-process.
    let gns_global = GnsGlobal::get().expect("no global networking state");
    let mut transport = Transport::new(gns_global.clone(), Ipv4Addr::LOCALHOST.into(), 27821).expect("connection failed");
    let tick_rate = Duration::from_millis(50);

    let bsp_name = "assets/box.bsp";
    let bsp = load_bsp(bsp_name);

    println!("Listening for connections...");

    loop {
        let now = Instant::now();
        gns_global.poll_callbacks();
        transport.poll_messages(message_handler);
        let elapsed = Instant::now() - now;

        if elapsed < tick_rate {
            std::thread::sleep(tick_rate - elapsed);
        }
    }
}

fn message_handler(_msg: Message) {
}
