use check::Check;
use chrono::serde::ts_milliseconds;
use chrono::{DateTime, TimeZone, Utc};
use clap::ArgMatches;
use serde::Serialize;
use transition::{Conditions, Transition};
use websocket::{ClientBuilder, Message, OwnedMessage};

use core::time;
use std::io::{stdin, Error};

use crate::check::{deserialize_item_checks, deserialize_location_checks};
use crate::output::{print_flags_toggled, print_transition, print_verbose_diff};
use crate::qusb::{attempt_qusb_connection, QusbRequestMessage};
use crate::snes::NamedAddresses;
use crate::transition::{deserialize_transitions_map, entrance_transition, overworld_transition};

use colored::*;
use std::borrow::{Borrow, Cow};
use std::collections::{HashMap, VecDeque};
use std::fs::File;

use csv::Writer;
use std::thread::sleep;

mod check;
mod explore;
pub mod output;
mod qusb;
mod serde_lttp;
mod snes;
mod transition;

/// Snes memory address
pub const VRAM_START: u32 = 0xf50000;
pub const SAVE_DATA_OFFSET: usize = 0xF000;
pub const SAVEDATA_START: u32 = VRAM_START + SAVE_DATA_OFFSET as u32;
/// I'm too lazy to manually translate dunka's values, so I'll just use this instead to read from the correct memory address
pub const DUNKA_VRAM_READ_OFFSET: u32 = SAVEDATA_START + 0x280;
pub const DUNKA_VRAM_READ_SIZE: u32 = 0x280;

/// Address keeping track of current overworld tile, remains at previous value when entering non-ow tile
pub const ADDRESS_OW_SLOT_INDEX: u32 = 0x7E040A;
/// Address keeping track of latest entrance transition, i.e. walking in or out of house/dungeon/etc
pub const ADDRESS_ENTRANCE_ID: u32 = 0x7E010E;
/// Address that's `1` if Link is inside, `0` if outside;
pub const ADDRESS_IS_INSIDE: u32 = 0x7E001B;

/// Struct used for serializing different types of checks into the same csv format.
/// Events include transitions, checking locations (e.g. chests), and getting items
#[derive(Serialize, Debug, PartialEq)]
pub struct Event {
    #[serde(with = "ts_milliseconds")]
    timestamp: DateTime<Utc>,
    indoors: Option<bool>,
    to: Option<String>,
    location_id: Option<String>,
    item_id: Option<String>,
}

impl From<&Transition> for Event {
    fn from(transition: &Transition) -> Self {
        Event {
            timestamp: transition
                .timestamp
                .expect("Found transition missing timestamp when serializing"),
            indoors: Some(transition.indoors),
            to: Some(transition.name.to_string()),
            location_id: None,
            item_id: None,
        }
    }
}

impl From<&mut Transition> for Event {
    fn from(transition: &mut Transition) -> Self {
        Event {
            timestamp: transition
                .timestamp
                .expect("Found transition missing timestamp when serializing"),
            indoors: Some(transition.indoors),
            to: Some(transition.name.to_string()),
            location_id: None,
            item_id: None,
        }
    }
}

impl<T> From<T> for Event
where
    T: Borrow<Check>,
{
    fn from(check: T) -> Self {
        let check: &Check = check.borrow();
        let timestamp = check
            .time_of_check
            .expect("Found check missing timestamp when serializing");
        if check.is_item && !check.is_progressive {
            Event {
                timestamp,
                indoors: None,
                to: None,
                location_id: None,
                item_id: Some(check.name.to_string()),
            }
        } else if check.is_item && check.is_progressive {
            Event {
                timestamp,
                indoors: None,
                to: None,
                location_id: None,
                item_id: Some(format!("{} - {}", check.name, check.progressive_level)),
            }
        } else {
            Event {
                timestamp,
                indoors: None,
                to: None,
                location_id: Some(check.name.to_string()),
                item_id: match &check.item {
                    Some(item) => Some(item.to_string()),
                    None => None,
                },
            }
        }
    }
}

impl Default for Event {
    fn default() -> Self {
        Self {
            timestamp: chrono::Utc.timestamp_millis(0),
            indoors: Default::default(),
            to: Default::default(),
            location_id: Default::default(),
            item_id: Default::default(),
        }
    }
}

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

    let mut connected = false;

    // As part of completing the connection, we need to find a Snes device to attach to.
    // We'll just attach to the first one we find, as most use cases will only have one connected snes device.
    while !connected {
        connected = attempt_qusb_connection(&mut client)?;
        sleep(time::Duration::from_millis(2000));
    }

    let mut responses: VecDeque<Vec<u8>> = VecDeque::new();
    let mut checks_responses: VecDeque<Vec<u8>> = VecDeque::new();
    let time_start = Utc::now();
    let csv_name = time_start.format("%Y%m%d_%H%M%S.csv").to_string();
    File::create(&csv_name)?;
    let mut writer = Writer::from_path(csv_name)?;

    let mut locations: Vec<Check> = deserialize_location_checks()?
        .into_iter()
        // 0 offset checks hasn't been given a proper value in checks.json yet
        .filter(|check| check.dunka_offset != 0)
        .collect();
    let mut items: Vec<Check> = deserialize_item_checks()?
        .into_iter()
        .filter(|check| check.dunka_offset != 0)
        .collect();
    let mut transitions: HashMap<SnesMemoryID, Transition> = deserialize_transitions_map()?;

    loop {
        // since we can't choose multiple addresses in a single request, we instead fetch a larger chunk of data from given address and forward
        // so we don't have to make multiple requests
        match get_address_request(&mut client, VRAM_START, 0x40B) {
            Ok(response) => {
                if let OwnedMessage::Binary(response) = response {
                    check_for_transitions(
                        &response,
                        verbosity,
                        &mut responses,
                        &mut transitions,
                        &mut writer,
                    )?;
                }
            }
            Err(e) => println!("Failed request: {:?}", e),
        };

        // Checks reading from the same address and chunk as dunka
        match get_dunka_chunka(&mut client) {
            Ok(response) => {
                check_for_location_checks(
                    &response,
                    verbosity,
                    &mut checks_responses,
                    &mut locations,
                    &mut writer,
                )?;
                check_for_item_checks(
                    &response,
                    verbosity,
                    &mut checks_responses,
                    &mut items,
                    &mut writer,
                )?;
                checks_responses.push_back(response);
            }
            Err(e) => println!("Failed request: {:?}", e),
        }

        // Only keep the last few responses to decrease memory usage
        if responses.len() > 60 {
            responses.pop_front();
        }
        if checks_responses.len() > 60 {
            checks_responses.pop_front();
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

/// Reads twice, guessing due to limitation of request sizes
fn get_dunka_chunka(
    client: &mut websocket::sync::Client<std::net::TcpStream>,
) -> anyhow::Result<Vec<u8>> {
    let first_message = &QusbRequestMessage::get_address(SAVEDATA_START, 0x280);
    let second_message = &QusbRequestMessage::get_address(DUNKA_VRAM_READ_OFFSET, 0x280);

    let message = Message {
        opcode: websocket::message::Type::Text,
        cd_status_code: None,
        payload: Cow::Owned(serde_json::to_vec(first_message)?),
    };
    let mut combined_result: Vec<u8> = Vec::new();
    client.send_message(&message)?;
    let response = client.recv_message()?;
    if let OwnedMessage::Binary(res) = response {
        combined_result.append(&mut res.clone());
    };

    let message = Message {
        opcode: websocket::message::Type::Text,
        cd_status_code: None,
        payload: Cow::Owned(serde_json::to_vec(second_message)?),
    };
    client.send_message(&message)?;
    let response = client.recv_message()?;
    if let OwnedMessage::Binary(res) = response {
        combined_result.append(&mut res.clone());
    };
    Ok(combined_result)
}

fn get_address_request(
    client: &mut websocket::sync::Client<std::net::TcpStream>,
    address: u32,
    size: usize,
) -> anyhow::Result<OwnedMessage> {
    let message = &QusbRequestMessage::get_address(address, size);

    let message = Message {
        opcode: websocket::message::Type::Text,
        cd_status_code: None,
        payload: Cow::Owned(serde_json::to_vec(message)?),
    };

    client.send_message(&message)?;
    let message = client.recv_message()?;
    Ok(message)
}

fn check_for_location_checks<U>(
    response: U,
    verbosity: u64,
    previous_values: &mut VecDeque<Vec<u8>>,
    checks: &mut Vec<Check>,
    writer: &mut Writer<File>,
) -> anyhow::Result<()>
where
    U: AsRef<[u8]>,
{
    let response = response.as_ref();

    for check in checks {
        let current_check_value = response[check.dunka_offset as usize];
        if previous_values.len() > 0
            && (previous_values[previous_values.len() - 1][check.dunka_offset as usize]
                != current_check_value)
        {
            let previous_value = &previous_values[previous_values.len() - 1];
            let previous_check_value = previous_value[check.dunka_offset as usize];
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

fn check_for_item_checks<U>(
    response: U,
    verbosity: u64,
    previous_values: &mut VecDeque<Vec<u8>>,
    checks: &mut Vec<Check>,
    writer: &mut Writer<File>,
) -> anyhow::Result<()>
where
    U: AsRef<[u8]>,
{
    let response = response.as_ref();

    for check in checks {
        let current_check_value = response[check.dunka_offset as usize];

        if previous_values.len() > 0
            && (previous_values[previous_values.len() - 1][check.dunka_offset as usize]
                != current_check_value)
        {
            let previous_value = &previous_values[previous_values.len() - 1];
            let previous_check_value = previous_value[check.dunka_offset as usize];
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
                writer.serialize(Event::from(check))?;
            } else if check.is_progressive && current_check_value > check.snes_value {
                check.progress_item(current_check_value);
                println!(
                    "Item get! time: {:?}, item: {}",
                    check.time_of_check,
                    format!("{} - {}", check.name, check.progressive_level).on_green(),
                );
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

fn check_for_transitions<U>(
    response: U,
    verbosity: u64,
    responses: &mut VecDeque<Vec<u8>>,
    transitions: &mut HashMap<SnesMemoryID, Transition>,
    writer: &mut Writer<File>,
) -> anyhow::Result<()>
where
    U: AsRef<[u8]>,
{
    let res = response.as_ref();

    match verbosity {
        1 => println!(
            "ow {}, indoors {}, entrance {}",
            res.overworld_tile(),
            res.indoors(),
            res.entrance_id()
        ),
        // If using level 2, you might wanna set a higher update interval, (e.g. --freq 10000 to update every 10 seconds) as it's A LOT of data
        2.. => {
            if responses.len() > 0 {
                print_verbose_diff(responses.get(responses.len() - 1).unwrap(), res);
                print_flags_toggled(responses.get(responses.len() - 1).unwrap(), res);
            } else {
                println!("Full response: {:?}", res)
            }
        }
        _ => (), // on 0 or somehow invalid verbosity level we don't do this logging as it's very spammy
    };

    if responses.len() > 0 {
        match responses.get(responses.len() - 1) {
            Some(previous_res) if overworld_transition(previous_res, &res) => {
                let mut transition = transitions
                    .get(&SnesMemoryID {
                        address_value: Some(res.overworld_tile() as u16),
                        indoors: Some(false),
                        ..Default::default()
                    })
                    .unwrap()
                    .clone();
                transition.time_transit();
                // let transition = Transition::new(res.overworld_tile() as u16, false);

                writer.serialize(Event::from(&transition))?;

                print_transition(&transition);
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
                let snes_id = SnesMemoryID {
                    address_value: Some(to as u16),
                    indoors: Some(res.indoors() == 1),
                    ..Default::default()
                };
                let mut transition = transitions
                    .get(&snes_id)
                    .ok_or(Error::new(
                        std::io::ErrorKind::NotFound,
                        format!(
                            "Couldn't find {:X} in transitions",
                            snes_id.address_value.unwrap()
                        ),
                    ))?
                    .clone();
                transition.time_transit();
                // let transition = Transition::new(to as u16, res.indoors() == 1);

                writer.serialize(Event::from(&transition))?;

                print_transition(&transition);
            }
            _ => (),
        }
    }
    responses.push_back(res.to_vec());

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    macro_rules! convert_to_event {
        ($($name:ident: $values:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (from_struct, expected) = $values;
                    assert_eq!(Event::from(&from_struct), expected)
                }
            )*
        };
    }

    convert_to_event! {
        from_location_check: (
            Check {
                name: "Mushroom".to_string(),
                address: 0x180013,
                dunka_offset: 0x411,
                dunka_mask: 0x10,
                time_of_check: Some(Utc.timestamp_millis(200)),
                ..Default::default()
            },
            Event {
                location_id: Some("Mushroom".to_string()),
                timestamp: Utc.timestamp_millis(200),
                ..Default::default()
            }
        ),
        from_normal_item_check: (
            Check {
                name: "Hookshot".to_string(),
                address: 0x0,
                dunka_offset: 0x342,
                dunka_mask: 0x01,
                time_of_check: Some(Utc.timestamp_millis(200)),
                is_item: true,
                ..Default::default()
            },
            Event {
                item_id: Some("Hookshot".to_string()),
                timestamp: Utc.timestamp_millis(200),
                ..Default::default()
            }
        ),
        from_progressive_item_check: (
            Check {
                name: "Progressive Sword".to_string(),
                address: 0x0,
                dunka_offset: 0x342,
                dunka_mask: 0x01,
                time_of_check: Some(Utc.timestamp_millis(200)),
                is_item: true,
                is_progressive: true,
                progressive_level: 3,
                ..Default::default()
            },
            Event {
                item_id: Some("Progressive Sword - 3".to_string()),
                timestamp: Utc.timestamp_millis(200),
                ..Default::default()
            }
        ),
        from_transition: (
            Transition {
                name: "Lala".to_string(),
                timestamp: Some(Utc.timestamp_millis(200)),
                ..Default::default()
            },
            Event {
                to: Some("Lala".to_string()),
                timestamp: Utc.timestamp_millis(200),
                indoors: Some(false),
                ..Default::default()
            }
        ),
    }
}
