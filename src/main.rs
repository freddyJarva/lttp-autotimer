use core::time;

use clap::{Arg, ArgMatches};
use colored::*;
use lttp_autotimer::qusb::{QusbRequestMessage, QusbResponseMessage};
use lttp_autotimer::snes::NamedAddresses;
use std::borrow::Cow;
use std::collections::VecDeque;
use std::fs::File;
use websocket::{ClientBuilder, Message, OwnedMessage};

use std::thread::sleep;

use chrono::Utc;
use csv::Writer;

use lttp_autotimer::{qusb, Transition, ADDRESS_IS_INSIDE};

fn main() -> anyhow::Result<()> {
    let matches = clap::App::new("Rando Auto Timer")
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
        ).arg(
            Arg::new("update frequency")
                .long("freq")
                .short('f')
                .about("Times per second the timer will check the snes memory for changes")
                .takes_value(true)
                .default_value("60")
        ).arg(
            Arg::new("verbose")
                .short('v')
        ).get_matches();
    connect_to_qusb(&matches)?;
    Ok(())
}

fn connect_to_qusb(args: &ArgMatches) -> anyhow::Result<()> {
    let host = args.value_of("host").unwrap();
    let port = args.value_of("port").unwrap();

    let update_frequency: u64 = args
        .value_of("update frequency")
        .unwrap()
        .parse()
        .expect("specified update frequency (--freq/-f) needs to be a positive integer");
    let sleep_time: u64 = 1000 / update_frequency;
    let verbose = args.is_present("verbose");
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
        sleep(time::Duration::from_millis(2000));
    }

    let mut responses: VecDeque<Vec<u8>> = VecDeque::new();
    let time_start = Utc::now();
    let csv_name = time_start.format("%Y%m%d_%H%M%S.csv").to_string();
    File::create(&csv_name)?;
    let mut writer = Writer::from_path(csv_name)?;

    loop {
        // since we can't choose multiple addresses in a single request, we instead fetch a larger chunk of data from given address and forward
        // so we don't have to make multiple requests
        let message = &QusbRequestMessage::get_address(ADDRESS_IS_INSIDE, 0x40B);

        let message = Message {
            opcode: websocket::message::Type::Text,
            cd_status_code: None,
            payload: Cow::Owned(serde_json::to_vec(message)?),
        };
        client.send_message(&message)?;

        match client.recv_message() {
            Ok(response) => match response {
                OwnedMessage::Binary(res) => {
                    if verbose {
                        println!(
                            "ow {}, indoors {}, entrance {}",
                            res.overworld_tile(),
                            res.indoors(),
                            res.entrance_id()
                        );
                    }

                    if responses.len() > 0 {
                        match responses.get(responses.len() - 1) {
                            Some(previous_res) if overworld_transition(previous_res, &res) => {
                                let transition =
                                    Transition::new(res.overworld_tile() as u16, false);

                                writer.serialize(&transition)?;
                                writer.flush()?;

                                println!(
                                    "Transition made!: time: {:?}, indoors: {:?}, to: {:X}",
                                    transition.timestamp, transition.indoors, transition.to
                                );
                            }
                            Some(previous_res) if entrance_transition(previous_res, &res) => {
                                let to;
                                if res.indoors() == 1 {
                                    // new position is inside
                                    to = res.entrance_id();
                                } else {
                                    // new position is outside
                                    to = res.overworld_tile();
                                }
                                let transition = Transition::new(to as u16, res.indoors() == 1);

                                writer.serialize(&transition)?;
                                writer.flush()?;

                                println!(
                                    "Transition made!: time: {:?}, indoors: {:?}, to: {:X}",
                                    transition.timestamp, transition.indoors, transition.to
                                );
                            }
                            _ => (),
                        }
                    }
                    responses.push_back(res);
                }
                _ => (),
            },
            Err(e) => println!("{:?}", e),
        }

        sleep(time::Duration::from_millis(sleep_time));
    }
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

fn overworld_transition(previous_res: &Vec<u8>, response: &Vec<u8>) -> bool {
    previous_res.overworld_tile() != response.overworld_tile()
}

fn entrance_transition(previous_res: &Vec<u8>, response: &Vec<u8>) -> bool {
    previous_res.indoors() != response.indoors()
}
