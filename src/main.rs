use core::time;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
use std::thread::sleep;

use lttp_autotimer::{deserialize_message, ADDRESS_OW_SLOT_INDEX, ADDRESS_OW_TILE_INDEX};

fn main() -> anyhow::Result<()> {
    let loopback = Ipv4Addr::new(127, 0, 0, 1);
    let socket = SocketAddrV4::new(loopback, 46700);
    let listener = TcpListener::bind(socket)?;
    let port = listener.local_addr()?;
    println!("Listening on {}, access this port to end the program", port);
    let (mut tcp_stream, addr) = listener.accept()?; //block  until requested
    println!("Connection received! {:?}.", addr);

    loop {
        tcp_stream.write(b"READ|0x7E040A,0x7E008A|2|System Bus\n")?;
        let mut buf = [0 as u8; 40];
        tcp_stream.read(&mut buf)?;
        let deserialized = deserialize_message(&buf)?;
        println!(
            "OW TILE: {:?}, SLOT: {:?}",
            deserialized.two_bytes(0),
            deserialized.two_bytes(2)
        );
        sleep(time::Duration::from_millis(16));
    }
}
