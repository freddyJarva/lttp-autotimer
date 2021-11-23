use core::time;
use std::cmp::max;
use std::collections::VecDeque;
use std::fs::File;

use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};

use std::thread::sleep;

use chrono::Utc;
use csv::Writer;
use lttp_autotimer::request::Response;

use lttp_autotimer::{
    read, Transition, ADDRESS_ENTRANCE_ID_U8, ADDRESS_IS_INSIDE_U8, ADDRESS_OW_SLOT_INDEX_U8,
};

fn main() -> anyhow::Result<()> {
    let loopback = Ipv4Addr::new(127, 0, 0, 1);
    let socket = SocketAddrV4::new(loopback, 46700);
    let listener = TcpListener::bind(socket)?;
    let port = listener.local_addr()?;
    println!("Listening on {}, access this port to end the program", port);
    let (mut tcp_stream, addr) = listener.accept()?; //block  until requested
    println!("Connection received! {:?}.", addr);

    let mut location_buf = [0 as u8; 40];

    let mut responses: VecDeque<Response> = VecDeque::new();
    let time_start = Utc::now();
    let csv_name = time_start.format("%Y%m%d_%H%M%S.csv").to_string();
    File::create(&csv_name)?;
    let mut writer = Writer::from_path(csv_name)?;

    loop {
        let response = read::overworld_location(
            &mut tcp_stream,
            &mut location_buf,
            vec![
                ADDRESS_OW_SLOT_INDEX_U8,
                ADDRESS_ENTRANCE_ID_U8,
                ADDRESS_IS_INSIDE_U8,
            ],
        )?;

        if responses.len() > 0 {
            match responses.get(responses.len() - 1) {
                Some(previous_res) if overworld_transition(previous_res, &response) => {
                    let transition = Transition::new(response.data[0] as u16, false);

                    writer.serialize(&transition)?;
                    writer.flush()?;

                    println!(
                        "Transition made!: time: {:?}, indoors: {:?}, to: {:X}",
                        transition.timestamp, transition.indoors, transition.to
                    );
                }
                Some(previous_res) if entrance_transition(previous_res, &response) => {
                    let to;
                    if response.data[2] == 1 {
                        // new position is inside
                        to = response.data[1];
                    } else {
                        // new position is outside
                        to = response.data[0];
                    }
                    let transition = Transition::new(to as u16, response.data[2] == 1);

                    writer.serialize(&transition)?;
                    writer.flush()?;

                    println!(
                        "Transition made!: time: {:?}, indoors: {:?}, to: {:X}",
                        transition.timestamp, transition.indoors, transition.to
                    );
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

fn overworld_transition(previous_res: &Response, response: &Response) -> bool {
    previous_res.data[0] != response.data[0]
}

fn entrance_transition(previous_res: &Response, response: &Response) -> bool {
    previous_res.data[2] != response.data[2]
}
