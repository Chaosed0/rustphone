mod transport;
use transport::Transport;

use gns::GnsGlobal;
use std::net::Ipv4Addr;
use std::time::Instant;
use std::time::Duration;
use shared::Message;

fn main() {
    // Initial the global networking state. Note that this instance must be unique per-process.
    let gns_global = GnsGlobal::get().expect("no global networking state");
    let mut transport = Transport::new(gns_global.clone(), Ipv4Addr::LOCALHOST.into(), 27821).expect("connection failed");
    let tick_rate = Duration::from_millis(50);

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
