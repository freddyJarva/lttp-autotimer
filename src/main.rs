use core::time;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
use std::thread::sleep;

use lttp_autotimer::{
    deserialize_message,
    request::{RequestBody, RequestType},
};

fn main() -> anyhow::Result<()> {
    let loopback = Ipv4Addr::new(127, 0, 0, 1);
    let socket = SocketAddrV4::new(loopback, 46700);
    let listener = TcpListener::bind(socket)?;
    let port = listener.local_addr()?;
    println!("Listening on {}, access this port to end the program", port);
    let (mut tcp_stream, addr) = listener.accept()?; //block  until requested
    println!("Connection received! {:?}.", addr);

    let mut body = RequestBody {
        request_type: RequestType::Read,
        addresses: vec![b"0x7E040A".to_vec(), b"0x7E008A".to_vec()],
        address_length: b'2',
        device_type: "System Bus".to_string(),
    };
    loop {
        tcp_stream.write(body.serialize().as_ref())?;
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
