use crate::event::{Event, EventEnum, EventLog, EventTracker};
use check::Check;

#[macro_use]
extern crate lazy_static;

use chrono::Utc;
use clap::ArgMatches;

use snes::SnesRam;
use transition::{Conditions, Tile};
use websocket::{ClientBuilder, Message, OwnedMessage};

use core::time;
use std::io::stdin;

use crate::check::{deserialize_item_checks, deserialize_location_checks};
use crate::output::{print_flags_toggled, print_transition, print_verbose_diff};
use crate::qusb::{attempt_qusb_connection, QusbRequestMessage};
use crate::snes::NamedAddresses;
use crate::transition::deserialize_transitions_map;

use colored::*;
use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::fs::File;

use csv::Writer;
use std::thread::sleep;

mod check;
mod event;
pub mod output;
mod qusb;
mod serde_lttp;
mod snes;

#[cfg(test)]
#[macro_use]
mod test_macros;

mod transition;

/// Snes memory address
pub const VRAM_START: u32 = 0xf50000;
pub const SAVE_DATA_OFFSET: usize = 0xF000;
pub const SAVEDATA_START: u32 = VRAM_START + SAVE_DATA_OFFSET as u32;
/// I'm too lazy to manually translate dunka's values, so I'll just use this instead to read from the correct memory address
pub const DUNKA_VRAM_READ_OFFSET: u32 = SAVEDATA_START + 0x280;
pub const DUNKA_VRAM_READ_SIZE: u32 = 0x280;

const DUNKA_START: usize = SAVEDATA_START as usize + 0x21;
const DUNKA_CHUNK_SIZE: usize = 0x3f1;
const DUNKA_OFFSET: usize = DUNKA_START - VRAM_START as usize;

const COORDINATE_OFFSET: usize = 0xc184;
const COORDINATE_CHUNK_SIZE: usize = 0x4;

const TILE_INFO_CHUNK_SIZE: usize = 0x40B;

/// Hashable id for map lookups
#[derive(Default, PartialEq, Hash, Eq, Debug)]
pub struct SnesMemoryID {
    pub address: Option<u32>,
    pub mask: Option<u8>,
    pub address_value: Option<u16>,
    pub indoors: Option<bool>,
    pub conditions: Option<Conditions>,
}

pub fn connect_to_qusb(args: &ArgMatches) -> anyhow::Result<()> {
    let host = args.value_of("host").unwrap();
    let port = args.value_of("port").unwrap();

    let update_frequency: u64 = args
        .value_of("update frequency")
        .unwrap()
        .parse()
        .expect("specified update frequency (--freq/-f) needs to be a positive integer");
    let verbosity = args.occurrences_of("v");
    println!("Verbosity level: {}", verbosity);
    let manual_update = args.is_present("manual update");

    println!(
        "{} to connect to {}:{}",
        "Attempting".green().bold(),
        host,
        port
    );
    let mut client = ClientBuilder::new(&format!("ws://{}:{}", host, port))?.connect_insecure()?;
    println!("{} to qusb!", "Connected".green().bold());

    // As part of completing the connection, we need to find a Snes device to attach to.
    // We'll just attach to the first one we find, as most use cases will only have one connected snes device.
    let mut connected = false;
    while !connected {
        connected = attempt_qusb_connection(&mut client)?;
        sleep(time::Duration::from_millis(2000));
    }

    let mut ram_history: VecDeque<SnesRam> = VecDeque::new();

    let time_start = Utc::now();
    let csv_name = time_start.format("%Y%m%d_%H%M%S.csv").to_string();
    File::create(&csv_name)?;
    let mut writer = Writer::from_path(csv_name)?;

    let mut events = EventTracker::new();

    let mut locations: Vec<Check> = deserialize_location_checks()?
        .into_iter()
        // 0 offset checks hasn't been given a proper value in checks.json yet
        .filter(|check| check.dunka_offset != 0)
        .collect();
    let mut items: Vec<Check> = deserialize_item_checks()?
        .into_iter()
        .filter(|check| check.dunka_offset != 0)
        .collect();

    loop {
        match get_chunka_chungus(&mut client) {
            Ok(snes_ram) => {
                check_for_transitions(
                    &snes_ram,
                    verbosity,
                    &mut ram_history,
                    &mut writer,
                    &mut events,
                )?;
                check_for_location_checks(
                    &snes_ram,
                    verbosity,
                    &mut ram_history,
                    &mut locations,
                    &mut writer,
                    &mut events,
                )?;
                check_for_item_checks(
                    &snes_ram,
                    verbosity,
                    &mut ram_history,
                    &mut items,
                    &mut writer,
                    &mut events,
                )?;
                ram_history.push_back(snes_ram);
            }
            Err(e) => println!("Failed request: {:?}", e),
        }

        // Only keep the last few responses to decrease memory usage
        if ram_history.len() > 60 {
            ram_history.pop_front();
        }

        writer.flush()?;

        if manual_update {
            println!("Press enter to update...");
            stdin()
                .read_line(&mut String::new())
                .ok()
                .expect("Failed to read line");
        } else {
            sleep(time::Duration::from_millis(update_frequency));
        }
    }
}

// since we can't choose multiple addresses in a single request, we instead fetch a larger chunk of data from given address and forward
// so we don't have to make multiple requests
fn get_chunka_chungus(
    client: &mut websocket::sync::Client<std::net::TcpStream>,
) -> anyhow::Result<SnesRam> {
    let tile_info_message = &QusbRequestMessage::get_address(VRAM_START, TILE_INFO_CHUNK_SIZE);
    let dunka_chunka_message =
        &QusbRequestMessage::get_address(DUNKA_START as u32, DUNKA_CHUNK_SIZE);
    let coordinate_message = &QusbRequestMessage::get_address(
        VRAM_START + COORDINATE_OFFSET as u32,
        COORDINATE_CHUNK_SIZE,
    );

    let mut snes_ram = SnesRam::new();

    // location checks + items
    let message = Message {
        opcode: websocket::message::Type::Text,
        cd_status_code: None,
        payload: Cow::Owned(serde_json::to_vec(dunka_chunka_message)?),
    };
    client.send_message(&message)?;
    let response = client.recv_message()?;
    if let OwnedMessage::Binary(res) = response {
        snes_ram.dunka_chunka = res;
    };

    // tiles
    let message = Message {
        opcode: websocket::message::Type::Text,
        cd_status_code: None,
        payload: Cow::Owned(serde_json::to_vec(tile_info_message)?),
    };
    client.send_message(&message)?;
    let response = client.recv_message()?;
    if let OwnedMessage::Binary(res) = response {
        snes_ram.tile_info_chunk = res;
    };

    // coordinates
    let message = Message {
        opcode: websocket::message::Type::Text,
        cd_status_code: None,
        payload: Cow::Owned(serde_json::to_vec(coordinate_message)?),
    };
    client.send_message(&message)?;
    let response = client.recv_message()?;
    if let OwnedMessage::Binary(res) = response {
        snes_ram.coordinate_chunk = res;
    };

    Ok(snes_ram)
}

fn check_for_location_checks(
    ram: &SnesRam,
    verbosity: u64,
    ram_history: &mut VecDeque<SnesRam>,
    checks: &mut Vec<Check>,
    writer: &mut Writer<File>,
    events: &mut EventTracker,
) -> anyhow::Result<()> {
    for check in checks {
        let current_check_value = ram.get_byte(check.dunka_offset as usize);
        if ram_history.len() > 0
            && (ram_history[ram_history.len() - 1].get_byte(check.dunka_offset as usize)
                != current_check_value)
        {
            let previous_state = &ram_history[ram_history.len() - 1];
            let previous_check_value = previous_state.get_byte(check.dunka_offset as usize);
            if verbosity > 0 {
                println!(
                    "{}: {} -> {} -- bitmask applied: {} -> {}",
                    check.name.on_blue(),
                    previous_check_value.to_string().red(),
                    current_check_value.to_string().green(),
                    (previous_check_value & check.dunka_mask).to_string().red(),
                    (current_check_value & check.dunka_mask).to_string().green()
                )
            } else if current_check_value & check.dunka_mask != 0 && !check.is_checked {
                check.mark_as_checked();
                println!(
                    "Check made! time: {:?}, location: {}",
                    check.time_of_check,
                    check.name.on_blue(),
                );
                events.push(EventEnum::LocationCheck(check.clone()));
                writer.serialize(Event::from(check))?;
            }
        } else {
            if verbosity > 0 {
                println!(
                    "{}: {} -- bitmask applied: {}",
                    check.name.on_blue(),
                    current_check_value,
                    current_check_value & check.dunka_mask
                )
            }
        }
    }

    Ok(())
}

fn check_for_item_checks(
    ram: &SnesRam,
    verbosity: u64,
    previous_values: &mut VecDeque<SnesRam>,
    checks: &mut Vec<Check>,
    writer: &mut Writer<File>,
    events: &mut EventTracker,
) -> anyhow::Result<()> {
    for check in checks {
        let current_check_value = ram.get_byte(check.dunka_offset as usize);

        if previous_values.len() > 0
            && (previous_values[previous_values.len() - 1].get_byte(check.dunka_offset as usize)
                != current_check_value)
        {
            let previous_state = &previous_values[previous_values.len() - 1];
            let previous_check_value = previous_state.get_byte(check.dunka_offset as usize);
            if verbosity > 0 {
                println!(
                    "{}: {} -> {} -- bitmask applied: {} -> {}",
                    check.name.on_blue(),
                    previous_check_value.to_string().red(),
                    current_check_value.to_string().green(),
                    (previous_check_value & check.dunka_mask).to_string().red(),
                    (current_check_value & check.dunka_mask).to_string().green()
                )
            } else if !check.is_progressive
                && current_check_value & check.dunka_mask != 0
                && !check.is_checked
            {
                check.mark_as_checked();
                println!(
                    "Item get! time: {:?}, item: {}",
                    check.time_of_check,
                    check.name.on_green(),
                );
                events.push(EventEnum::ItemGet(check.clone()));
                writer.serialize(Event::from(check))?;
            } else if check.is_progressive && current_check_value > check.snes_value {
                check.progress_item(current_check_value);
                println!(
                    "Item get! time: {:?}, item: {}",
                    check.time_of_check,
                    format!("{} - {}", check.name, check.progressive_level).on_green(),
                );
                events.push(EventEnum::ItemGet(check.clone()));
                writer.serialize(Event::from(check))?;
            }
        } else {
            if verbosity > 0 {
                println!(
                    "{}: {} -- bitmask applied: {}",
                    check.name.on_blue(),
                    current_check_value,
                    current_check_value & check.dunka_mask
                )
            }
        }
    }

    Ok(())
}

fn check_for_transitions(
    ram: &SnesRam,
    verbosity: u64,
    ram_history: &mut VecDeque<SnesRam>,
    writer: &mut Writer<File>,
    events: &mut EventTracker,
) -> anyhow::Result<()> {
    match verbosity {
        1 => println!(
            "ow {}, indoors {}, entrance {}",
            ram.tile_info_chunk.overworld_tile(),
            ram.tile_info_chunk.indoors(),
            ram.tile_info_chunk.entrance_id()
        ),
        // If using level 2, you might wanna set a higher update interval, (e.g. --freq 10000 to update every 10 seconds) as it's A LOT of data
        2.. => {
            if ram_history.len() > 0 {
                print_verbose_diff(
                    &ram_history
                        .get(ram_history.len() - 1)
                        .unwrap()
                        .tile_info_chunk,
                    &ram.tile_info_chunk,
                );
                print_flags_toggled(
                    &ram_history
                        .get(ram_history.len() - 1)
                        .unwrap()
                        .tile_info_chunk,
                    &ram.tile_info_chunk,
                );
            } else {
                println!("Full response: {:?}", ram.tile_info_chunk)
            }
        }
        _ => (), // on 0 or somehow invalid verbosity level we don't do this logging as it's very spammy
    };

    // Use events if one transition has been triggered.
    match events.latest_transition() {
        Some(previous_transition) => {
            if let Ok(mut current_tile) = Tile::try_from_ram(ram, &previous_transition) {
                if current_tile.name != previous_transition.name {
                    current_tile.time_transit();
                    writer.serialize(Event::from(&current_tile))?;
                    print_transition(&current_tile);
                    events.push(EventEnum::Transition(current_tile));
                }
            }
        }
        None => {
            panic!("You've reached the unreachable, as EventTracker should always contain a transition when using ::new");
        }
    }

    Ok(())
}
