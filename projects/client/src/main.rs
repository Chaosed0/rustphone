mod transport;
use transport::Transport;

use raylib::prelude::*;
use gns::GnsGlobal;
use std::net::Ipv4Addr;
use shared::Message;

fn main() {
    // Initial the global networking state. Note that this instance must be unique per-process.
    let gns_global = GnsGlobal::get().expect("no global networking state");

    let (mut rl, thread) = raylib::init()
        .size(640, 480)
        .title("Hello, World")
        .build();

    let transport = Transport::new(gns_global.clone(), Ipv4Addr::LOCALHOST.into(), 27821).expect("connection failed");

    while !rl.window_should_close() {
        let mut d = rl.begin_drawing(&thread);

        transport.poll_messages(message_handler);
        gns_global.poll_callbacks();

        d.clear_background(Color::WHITE);
        d.draw_text("Hello, world!", 12, 12, 20, Color::BLACK);
    }
}

fn message_handler(_msg: Message) {
}
