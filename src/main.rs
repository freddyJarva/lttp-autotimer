use core::time;

use clap::{Arg, ArgMatches};
use colored::*;
use lttp_autotimer::qusb::{attempt_qusb_connection, QusbRequestMessage};
use lttp_autotimer::snes::NamedAddresses;
use lttp_autotimer::transition::{entrance_transition, overworld_transition, Transition};
use std::borrow::Cow;
use std::collections::VecDeque;
use std::fs::File;
use websocket::{ClientBuilder, Message, OwnedMessage};

use std::thread::sleep;

use chrono::Utc;
use csv::Writer;

use lttp_autotimer::ADDRESS_IS_INSIDE;

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
        connected = attempt_qusb_connection(&mut client)?;
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
