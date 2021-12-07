use crate::condition::Conditions;
use crate::event::{Event, EventEnum, EventLog, EventTracker};
use check::Check;

#[macro_use]
extern crate lazy_static;

use chrono::Utc;
use clap::ArgMatches;

use condition::{coordinate_condition_met, current_tile_condition_met};
use snes::SnesRam;
use tile::Tile;
use websocket::{ClientBuilder, Message, OwnedMessage};

use core::time;
use std::io::stdin;

use crate::check::{
    deserialize_event_checks, deserialize_item_checks, deserialize_location_checks,
};
use crate::output::StdoutPrinter;
use crate::qusb::{attempt_qusb_connection, QusbRequestMessage};
use crate::snes::NamedAddresses;

use colored::*;
use std::borrow::Cow;
use std::collections::VecDeque;
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

mod condition;
mod tile;

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

const GAME_STATS_OFFSET: usize = 0xf42d;
const GAME_STATS_SIZE: usize = 0x1f;

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
    let _verbosity = args.occurrences_of("v");
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

    let allow_output = match is_race_rom(&mut client) {
        Ok(race_rom) => {
            if race_rom {
                false
            } else {
                args.is_present("Non race mode")
            }
        }
        Err(_) => {
            println!(
                "Wasn't able to tell if race rom or not, defaulting to not allowing any event output"
            );
            false
        }
    };
    if !allow_output {
        println!(
            "{}: no game info will be output in this window.\nNOTE: THIS TOOL IS NOT RACE LEGAL DESPITE VISUAL OUTPUT BEING TURNED OFF.",
            "Race mode activated".red(),
        )
    }
    let print = StdoutPrinter::new(allow_output);

    let mut ram_history: VecDeque<SnesRam> = VecDeque::new();

    let time_start = Utc::now();
    let csv_name = time_start.format("%Y%m%d_%H%M%S.csv").to_string();
    File::create(&csv_name)?;
    let mut writer = Writer::from_path(csv_name)?;

    let mut events = EventTracker::new();

    let mut game_finished = false;
    // Intro/start screen counts as not started. Having selected a spawn point counts as game started.
    // This is to ensure it only checks for events - especially transitions - while in-game.
    let mut game_started = args.is_present("game started");

    let mut subscribed_events: Vec<Check> = deserialize_event_checks()?;
    let mut locations: Vec<Check> = deserialize_location_checks()?
        .into_iter()
        // 0 offset checks without conditions hasn't been given a proper value in checks.json yet
        .filter(|check| check.sram_offset != 0 || check.conditions.is_some())
        .collect();
    let mut items: Vec<Check> = deserialize_item_checks()?
        .into_iter()
        .filter(|check| check.sram_offset != 0)
        .collect();

    while !game_finished {
        match get_chunka_chungus(&mut client) {
            Ok(snes_ram) => {
                if !game_started {
                    game_started = game_has_started(&snes_ram);
                } else {
                    game_started = check_for_events(
                        &snes_ram,
                        &mut ram_history,
                        &mut subscribed_events,
                        &mut writer,
                        &mut events,
                        &print,
                    )?;
                    if game_started {
                        check_for_transitions(&snes_ram, &mut writer, &mut events, &print)?;
                        check_for_location_checks(
                            &snes_ram,
                            &mut ram_history,
                            &mut locations,
                            &mut writer,
                            &mut events,
                            &print,
                        )?;
                        check_for_item_checks(
                            &snes_ram,
                            &mut ram_history,
                            &mut items,
                            &mut writer,
                            &mut events,
                            &print,
                        )?;
                    }
                    ram_history.push_back(snes_ram);
                }
            }
            Err(e) => println!("Failed request: {:?}", e),
        }

        // Only keep the last few responses to decrease memory usage
        if ram_history.len() > 60 {
            ram_history.pop_front();
        }

        writer.flush()?;

        if events
            .latest_other_event()
            .map(|event| event.id == 5)
            .unwrap_or(false)
            || events
                .latest_transition()
                .map(|tile| tile.id == 556)
                .unwrap_or(false)
        {
            game_finished = true
        }
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

    println!("You defeated Ganon, Hurray! Press enter to exit...");
    stdin()
        .read_line(&mut String::new())
        .ok()
        .expect("Failed to read line");
    Ok(())
}

fn is_race_rom(client: &mut websocket::sync::Client<std::net::TcpStream>) -> anyhow::Result<bool> {
    loop {
        let message = &QusbRequestMessage::get_address(0x180213, 1);
        let message = Message {
            opcode: websocket::message::Type::Text,
            cd_status_code: None,
            payload: Cow::Owned(serde_json::to_vec(message)?),
        };
        client.send_message(&message)?;
        let response = client.recv_message()?;
        if let OwnedMessage::Binary(res) = response {
            return Ok(res[0] == 1 as u8);
        };
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
    let game_stats_message =
        &QusbRequestMessage::get_address(VRAM_START + GAME_STATS_OFFSET as u32, GAME_STATS_SIZE);

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

    // dungeon chest counters
    let message = Message {
        opcode: websocket::message::Type::Text,
        cd_status_code: None,
        payload: Cow::Owned(serde_json::to_vec(game_stats_message)?),
    };
    client.send_message(&message)?;
    let response = client.recv_message()?;
    if let OwnedMessage::Binary(res) = response {
        snes_ram.game_stats_chunk = res;
    };

    Ok(snes_ram)
}

fn check_for_location_checks(
    ram: &SnesRam,
    ram_history: &mut VecDeque<SnesRam>,
    checks: &mut Vec<Check>,
    writer: &mut Writer<File>,
    events: &mut EventTracker,
    print: &StdoutPrinter,
) -> anyhow::Result<()> {
    for check in checks {
        match &check.conditions {
            Some(conditions) => {
                if conditions.iter().all(|c| match c {
                    Conditions::PreviousTile(condition) => {
                        let previous_tile = &events
                            .latest_transition()
                            .expect("Transition should always exist");
                        current_tile_condition_met(condition, previous_tile)
                    }
                    Conditions::Coordinates { coordinates } => {
                        coordinate_condition_met(&coordinates, ram)
                    }
                    Conditions::Underworld => ram.indoors() == 1,
                    Conditions::DungeonCounterIncreased { sram_offset } => {
                        if ram_history.len() > 0 {
                            ram.get_byte(*sram_offset)
                                > ram_history[ram_history.len() - 1].get_byte(*sram_offset)
                        } else {
                            false
                        }
                    }
                }) {
                    check.mark_as_checked();

                    let location_check_event = EventEnum::LocationCheck(check.clone());
                    writer.serialize(Event::from(&location_check_event))?;
                    events.push(location_check_event);
                }
            }
            None => {
                let current_check_value = ram.get_byte(check.sram_offset as usize);
                if ram_history.len() > 0
                    && (ram_history[ram_history.len() - 1].get_byte(check.sram_offset as usize)
                        != current_check_value)
                {
                    if current_check_value & check.sram_mask != 0 && !check.is_checked {
                        check.mark_as_checked();
                        print.location_check(check);
                        let location_check_event = EventEnum::LocationCheck(check.clone());
                        writer.serialize(Event::from(&location_check_event))?;
                        events.push(location_check_event);
                    }
                }
            }
        }
    }

    Ok(())
}

fn check_for_item_checks(
    ram: &SnesRam,
    previous_values: &mut VecDeque<SnesRam>,
    checks: &mut Vec<Check>,
    writer: &mut Writer<File>,
    events: &mut EventTracker,
    print: &StdoutPrinter,
) -> anyhow::Result<()> {
    for check in checks {
        let current_check_value = ram.get_byte(check.sram_offset as usize);

        if previous_values.len() > 0
            && (previous_values[previous_values.len() - 1].get_byte(check.sram_offset as usize)
                != current_check_value)
        {
            if !check.is_progressive
                && current_check_value & check.sram_mask != 0
                && !check.is_checked
            {
                check.mark_as_checked();
                print.item_check(check);

                let item_event = EventEnum::ItemGet(check.clone());
                writer.serialize(Event::from(&item_event))?;
                events.push(item_event);
            } else if check.is_progressive && current_check_value > check.snes_value {
                check.progress_item(current_check_value);
                print.item_check(check);

                let item_event = EventEnum::ItemGet(check.clone());
                writer.serialize(Event::from(&item_event))?;
                events.push(item_event);
            }
        }
    }

    Ok(())
}

fn check_for_transitions(
    ram: &SnesRam,
    writer: &mut Writer<File>,
    events: &mut EventTracker,
    print: &StdoutPrinter,
) -> anyhow::Result<()> {
    // Use events if one transition has been triggered.
    match events.latest_transition() {
        Some(previous_transition) => {
            if let Ok(mut current_tile) = Tile::try_from_ram(ram, &previous_transition) {
                if current_tile.name != previous_transition.name {
                    current_tile.time_transit();
                    print.transition(&current_tile);
                    let transition_event = EventEnum::Transition(current_tile);
                    writer.serialize(Event::from(&transition_event))?;
                    events.push(transition_event);
                }
            }
        }
        None => {
            panic!("You've reached the unreachable, as EventTracker should always contain a transition when using ::new");
        }
    }

    Ok(())
}

/// Reads the ram value to see if has started since boot/reset/S&Q
///
/// Reads the value at 0x7e0010, which can be any of these:
///
/// * 00 - Intro
/// * 01 - File Select
/// * 02 - Copy File
/// * 03 - Delete File
/// * 04 - Name File
/// * 05 - Load File
/// * 06 - UnderworldLoad
/// * 07 - Underworld
/// * 08 - OverworldLoad
/// * 0A - OverworldSpecialLoad
/// * 0B - OverworldSpecial
/// * 0C/0D - Unused
/// * 0E - Interface
/// * 0F - SpotlightClose
/// * 10 - SpotlightOpen
/// * 11 - UnderworldFallingEntrance
/// * 12 - GameOver
/// * 13 - BossVictory_Pendant
/// * 14 - Attract
/// * 15 - MirrorWarpFromAge
/// * 16 - BossVictory_Crystal
/// * 17 - SaveAndQuit
/// * 18 - GanonEmerges
/// * 19 - TriforceRoom
/// * 1A - Credits
/// * 1B - SpawnSelect
fn game_has_started(ram: &SnesRam) -> bool {
    match ram.get_byte(0x10) {
        0x06..=0x0b => true,
        _ => false,
    }
}

fn check_for_events(
    ram: &SnesRam,
    previous_values: &mut VecDeque<SnesRam>,
    subscribed_events: &mut Vec<Check>,
    writer: &mut Writer<File>,
    events: &mut EventTracker,
    print: &StdoutPrinter,
) -> anyhow::Result<bool> {
    for event in subscribed_events {
        let current_event_value = ram.get_byte(event.sram_offset as usize);

        if previous_values.len() > 0
            && (previous_values[previous_values.len() - 1].get_byte(event.sram_offset as usize)
                != current_event_value)
        {
            if !event.is_progressive
                && current_event_value & event.sram_mask != 0
                && !event.is_checked
            {
                event.mark_as_checked();
                print.event(event);
                let occurred_event = EventEnum::Other(event.clone());
                writer.serialize(Event::from(&occurred_event))?;
                events.push(occurred_event);
            } else if event.is_progressive && current_event_value > event.snes_value {
                event.progress_item(current_event_value);
                print.event(event);
                let occurred_event = EventEnum::Other(event.clone());
                writer.serialize(Event::from(&occurred_event))?;
                events.push(occurred_event);
                return Ok(event.name != "Save & Quit");
            }
        }
    }
    Ok(true)
}
