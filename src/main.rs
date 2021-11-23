use core::time;

use clap::{Arg, ArgMatches};
use colored::*;
use lttp_autotimer::qusb::{QusbRequestMessage, QusbResponseMessage};
use std::borrow::Cow;
use std::collections::VecDeque;
use std::fs::File;
use websocket::{ClientBuilder, Message, OwnedMessage};

use std::net::{Ipv4Addr, SocketAddrV4, TcpListener};

use std::thread::sleep;

use chrono::Utc;
use csv::Writer;
use lttp_autotimer::request::Response;

use lttp_autotimer::{
    qusb, read, Transition, ADDRESS_ENTRANCE_ID_U8, ADDRESS_IS_INSIDE_U8, ADDRESS_OW_SLOT_INDEX_U8,
};

fn main() -> anyhow::Result<()> {
    let matches = clap::App::new("Rando Auto Timer")
        .arg(
            Arg::new("luabridge")
                .long("lua-bridge")
                .about("Use lua bridge (from lua folder in this repo) to connect instead of qusb.")
                .takes_value(false),
        )
        .arg(
            Arg::new("host")
                .long("host")
                .short('h')
                .about("url to server/localhost. When running locally the default value should be fine.")
                .takes_value(true)
                .default_value("127.0.0.1"),
        )
        .arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .about("port that websocket server is listening on. For qusb it's most likely 8080")
                .takes_value(true)
                .default_value("8080"),
        ).get_matches();
    if matches.is_present("luabridge") {
        connect_to_lua(&matches)?;
    } else {
        connect_to_qusb(&matches)?;
    }
    Ok(())
}

fn connect_to_qusb(args: &ArgMatches) -> anyhow::Result<()> {
    let host = args.value_of("host").unwrap();
    let port = args.value_of("port").unwrap();
    println!(
        "{} to connect to {}:{}",
        "Attempting".green().bold(),
        host,
        port
    );
    let mut client = ClientBuilder::new(&format!("ws://{}:{}", host, port))?.connect_insecure()?;
    println!("{} to qusb!", "Connected".green().bold());

    let mut connected = false;

    // As part of completing the connection, we need to find a Snes device to attach to.
    // We'll just attach to the first one we find, as most use cases will only have one connected snes device.
    while !connected {
        attempt_qusb_connection(&mut client, &mut connected)?;

        // let mut device_list_buf = [0 as u8; 100];

        // let v: serde_json::Value = serde_json::from_slice(&device_list_buf)?;
        // println!("{:?}", v);
        sleep(time::Duration::from_millis(2000));
    }
    Ok(())

    // let mut location_buf = [0 as u8; 40];
    // let mut responses: VecDeque<Response> = VecDeque::new();
    // let time_start = Utc::now();
    // let csv_name = time_start.format("%Y%m%d_%H%M%S.csv").to_string();
    // File::create(&csv_name)?;
    // let mut writer = Writer::from_path(csv_name)?;
    // loop {
    //     let response = read::current_location(
    //         &mut tcp_stream,
    //         &mut location_buf,
    //         vec![
    //             ADDRESS_OW_SLOT_INDEX_U8,
    //             ADDRESS_ENTRANCE_ID_U8,
    //             ADDRESS_IS_INSIDE_U8,
    //         ],
    //     )?;

    //     if responses.len() > 0 {
    //         match responses.get(responses.len() - 1) {
    //             Some(previous_res) if overworld_transition(previous_res, &response) => {
    //                 let transition = Transition::new(response.data[0] as u16, false);

    //                 writer.serialize(&transition)?;
    //                 writer.flush()?;

    //                 println!(
    //                     "Transition made!: time: {:?}, indoors: {:?}, to: {:X}",
    //                     transition.timestamp, transition.indoors, transition.to
    //                 );
    //             }
    //             Some(previous_res) if entrance_transition(previous_res, &response) => {
    //                 let to;
    //                 if response.data[2] == 1 {
    //                     // new position is inside
    //                     to = response.data[1];
    //                 } else {
    //                     // new position is outside
    //                     to = response.data[0];
    //                 }
    //                 let transition = Transition::new(to as u16, response.data[2] == 1);

    //                 writer.serialize(&transition)?;
    //                 writer.flush()?;

    //                 println!(
    //                     "Transition made!: time: {:?}, indoors: {:?}, to: {:X}",
    //                     transition.timestamp, transition.indoors, transition.to
    //                 );
    //             }
    //             _ => (),
    //         };
    //     }

    //     // Clean up code below
    //     responses.push_back(response);
    //     if responses.len() > 60 {
    //         responses.pop_front();
    //     }
    //     // Clear it for the next message
    //     location_buf.fill(0);
    //     sleep(time::Duration::from_millis(16));
    // }
}

fn attempt_qusb_connection(
    client: &mut websocket::sync::Client<std::net::TcpStream>,
    connected: &mut bool,
) -> Result<(), anyhow::Error> {
    let qusb_message = serde_json::to_vec(&QusbRequestMessage::device_list())?;
    let message = Message {
        opcode: websocket::message::Type::Text,
        cd_status_code: None,
        payload: Cow::Owned(qusb_message),
    };
    client.send_message(&message)?;
    Ok(
        if let OwnedMessage::Text(response) = client.recv_message()? {
            let devices: qusb::QusbResponseMessage = serde_json::from_str(&response)?;
            println!("{:?}", &devices);

            match devices.results.get(0) {
                Some(device) => {
                    println!(
                        "{} to the first option in devices: {}",
                        "Attaching".green().bold(),
                        &device
                    );
                    let message = Message {
                        opcode: websocket::message::Type::Text,
                        cd_status_code: None,
                        payload: Cow::Owned(serde_json::to_vec(&QusbRequestMessage::attach_to(
                            &device,
                        ))?),
                    };
                    client.send_message(&message)?;

                    let message = Message {
                        opcode: websocket::message::Type::Text,
                        cd_status_code: None,
                        payload: Cow::Owned(serde_json::to_vec(&QusbRequestMessage::device_info(
                            &device,
                        ))?),
                    };
                    client.send_message(&message)?;
                    match client.recv_message()? {
                        OwnedMessage::Text(message) => {
                            println!(
                                "{:?}",
                                serde_json::from_str::<QusbResponseMessage>(&message)?
                            )
                        }
                        _ => (),
                    };
                    *connected = true;
                    println!("{}", "Attached!".green().bold());
                }
                None => todo!(),
            }
        },
    )
}

fn connect_to_lua(args: &ArgMatches) -> anyhow::Result<()> {
    let loopback = Ipv4Addr::new(127, 0, 0, 1);
    let socket = SocketAddrV4::new(loopback, 46700);
    let listener = TcpListener::bind(socket)?;
    let port = listener.local_addr()?;
    println!("Listening on {}, access this port to end the program", port);
    let (mut tcp_stream, addr) = listener.accept()?;
    println!("Connection received! {:?}.", addr);
    let mut location_buf = [0 as u8; 40];
    let mut responses: VecDeque<Response> = VecDeque::new();
    let time_start = Utc::now();
    let csv_name = time_start.format("%Y%m%d_%H%M%S.csv").to_string();
    File::create(&csv_name)?;
    let mut writer = Writer::from_path(csv_name)?;
    loop {
        let response = read::current_location(
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
