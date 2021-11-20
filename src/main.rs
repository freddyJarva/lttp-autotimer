use core::time;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
use std::thread::sleep;

use lttp_autotimer::deserialize_message;

fn main() -> anyhow::Result<()> {
    let loopback = Ipv4Addr::new(127, 0, 0, 1);
    let socket = SocketAddrV4::new(loopback, 46700);
    let listener = TcpListener::bind(socket)?;
    let port = listener.local_addr()?;
    println!("Listening on {}, access this port to end the program", port);
    let (mut tcp_stream, addr) = listener.accept()?; //block  until requested
    println!("Connection received! {:?} is sending data.", addr);

    // while true {
    //     tcp_stream.
    // }
    loop {
        tcp_stream.write(b"Read|0x7ec184|2|System Bus\n")?;
        let mut buf = [0 as u8; 20];
        tcp_stream.read(&mut buf)?;
        println!("res: {:?}", deserialize_message(&buf)?.two_bytes());
        sleep(time::Duration::from_secs(1));
    }
}
