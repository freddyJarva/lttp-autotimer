use core::time;
use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};
use std::path::PathBuf;
use std::thread::sleep;

use chrono::Utc;
use csv::Writer;
use lttp_autotimer::request::Response;
use lttp_autotimer::{
    deserialize_message,
    request::{RequestBody, RequestType},
};
use lttp_autotimer::{read, request, Transition, VRAM_END, VRAM_START};

fn main() -> anyhow::Result<()> {
    let loopback = Ipv4Addr::new(127, 0, 0, 1);
    let socket = SocketAddrV4::new(loopback, 46700);
    let listener = TcpListener::bind(socket)?;
    let port = listener.local_addr()?;
    println!("Listening on {}, access this port to end the program", port);
    let (mut tcp_stream, addr) = listener.accept()?; //block  until requested
    println!("Connection received! {:?}.", addr);

    // let mut body = RequestBody {
    //     request_type: RequestType::Read,
    //     addresses: vec![b"0x7E040A".to_vec(), b"0x7E008A".to_vec()],
    //     address_length: b'2',
    //     device_type: "System Bus".to_string(),
    // };
    let mut location_buf = [0 as u8; 40];
    let mut vram_dump_buf = [0 as u8; (VRAM_END - VRAM_START + 20) as usize];

    let mut responses: VecDeque<Response> = VecDeque::new();
    let time_start = Utc::now();
    let csv_name = time_start.format("%Y%m%d_%H%M%S.csv").to_string();
    File::create(&csv_name)?;
    let mut writer = Writer::from_path(csv_name)?;

    loop {
        let response =
            read::overworld_location(&mut tcp_stream, &mut location_buf, vec![b"0x7E040A"])?;
        // request::two_byte_addresses(&mut tcp_stream, &mut buf, vec![b"0x7E040A", b"0x7E008A"])?;
        // let deserialized = deserialize_message(&buf)?;
        // println!(
        //     "OW TILE: {:?}, SLOT: {:?}",
        //     response.two_bytes(0),
        //     response.two_bytes(2)
        // );
        if responses.len() > 0 {
            match responses.get(responses.len() - 1) {
                Some(previous_res) if previous_res.two_bytes(0) != response.two_bytes(0) => {
                    let transition =
                        Transition::new(previous_res.two_bytes(0), response.two_bytes(0));

                    writer.serialize(&transition)?;
                    writer.flush()?;

                    println!("Transition made!: {:?}", transition);
                }
                _ => (),
            };
        }

        // Clean up code below
        responses.push_back(response);
        if responses.len() > 60 {
            responses.pop_front();
        }
        // Clear it for the next message
        location_buf.fill(0);
        sleep(time::Duration::from_millis(16));
    }
}
