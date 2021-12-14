use crate::condition::Conditions;
use crate::event::{Event, EventEnum, EventLog, EventTracker};
use check::Check;

#[macro_use]
extern crate lazy_static;

use chrono::{DateTime, Utc};
use clap::ArgMatches;

use condition::{
    coordinate_condition_met, current_tile_condition_met, dungeon_counter_condition_met,
    ram_value_change_condition_met,
};
use qusb::connect;
use snes::SnesRam;
use tile::Tile;
use websocket::{Message, OwnedMessage};

use core::time;
use std::io::stdin;
use std::sync::{mpsc, Arc, Mutex};
use std::time::Instant;

use crate::check::{
    deserialize_event_checks, deserialize_item_checks, deserialize_location_checks,
};
use crate::output::StdoutPrinter;
use crate::qusb::{init_meta_data, QusbRequestMessage};
use crate::snes::NamedAddresses;

use std::borrow::Cow;
use std::collections::VecDeque;
use std::fs::File;

use csv::Writer;
use std::thread::{self, sleep};

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

const GAME_STATS_OFFSET: usize = 0xf418;
const GAME_STATS_SIZE: usize = 0x39;

#[derive(Default)]
pub struct CliConfig {
    host: String,
    port: String,
    non_race_mode: bool,
    manual_update: bool,
    update_frequency: u64,
    _verbosity: u64,
}

pub fn connect_to_qusb(args: &ArgMatches) -> anyhow::Result<()> {
    let cli_config = Arc::new(Mutex::new(CliConfig {
        host: args.value_of("host").unwrap().to_string(),
        port: args.value_of("port").unwrap().to_string(),
        non_race_mode: args.is_present("Non race mode"),
        manual_update: args.is_present("manual update"),
        update_frequency: args
            .value_of("update frequency")
            .unwrap()
            .parse()
            .expect("specified update frequency (--freq/-f) needs to be a positive integer"),
        _verbosity: args.occurrences_of("v"),
    }));

    let (tx, rx) = mpsc::channel();

    let allow_output = Arc::new(Mutex::new(false));
    let game_finished = Arc::new(Mutex::new(false));

    let mut client = connect(Arc::clone(&cli_config))?;
    init_meta_data(
        &mut client,
        Arc::clone(&cli_config),
        Arc::clone(&allow_output),
    )?;
    read_snes_ram(
        tx,
        client,
        Arc::clone(&cli_config),
        Arc::clone(&game_finished),
    );

    let mut print = StdoutPrinter::new(*allow_output.lock().unwrap());

    let mut ram_history: VecDeque<SnesRam> = VecDeque::new();

    let csv_name = Utc::now().format("%Y%m%d_%H%M%S.csv").to_string();
    File::create(&csv_name)?;
    let mut writer = Writer::from_path(csv_name)?;

    let mut events = EventTracker::new();

    // Intro/start screen counts as not started. Having selected a spawn point counts as game started.
    // This is to ensure it only checks for events - especially transitions - while in-game.
    let mut game_started = false;

    let mut subscribed_events: Vec<Check> = deserialize_event_checks()?;
    let mut locations: Vec<Check> = deserialize_location_checks()?
        .into_iter()
        // 0 offset checks without conditions hasn't been given a proper value in checks.json yet
        .filter(|check| check.sram_offset.unwrap_or_default() != 0 || check.conditions.is_some())
        .collect();
    let mut items: Vec<Check> = deserialize_item_checks()?
        .into_iter()
        .filter(|check| check.sram_offset.unwrap_or_default() != 0)
        .collect();

    for (time_of_read, snes_ram) in rx {
        if !game_started {
            game_started = snes_ram.game_has_started();
        } else {
            game_started = check_for_events(
                &snes_ram,
                &mut ram_history,
                &mut subscribed_events,
                &mut writer,
                &mut events,
                &mut print,
                &time_of_read,
            )?;
            if game_started {
                check_for_transitions(
                    &snes_ram,
                    &mut writer,
                    &mut events,
                    &mut print,
                    &time_of_read,
                )?;
                check_for_location_checks(
                    &snes_ram,
                    &mut ram_history,
                    &mut locations,
                    &mut writer,
                    &mut events,
                    &mut print,
                    &time_of_read,
                )?;
                check_for_item_checks(
                    &snes_ram,
                    &mut ram_history,
                    &mut items,
                    &mut writer,
                    &mut events,
                    &mut print,
                    &time_of_read,
                )?;
            }
            ram_history.push_back(snes_ram);
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
            *game_finished.lock().unwrap() = true
        }
    }

    // This code should prooobably only execute when game_finished == true
    println!("You defeated Ganon, Hurray! Press enter to exit...");
    stdin()
        .read_line(&mut String::new())
        .ok()
        .expect("Failed to read line");
    Ok(())
}

pub fn read_snes_ram(
    tx: mpsc::Sender<(DateTime<Utc>, SnesRam)>,
    mut client: websocket::sync::Client<std::net::TcpStream>,
    config: Arc<Mutex<CliConfig>>,
    game_finished: Arc<Mutex<bool>>,
) {
    thread::spawn(move || -> anyhow::Result<()> {
        let cfg = config.lock().unwrap();
        let update_freq = time::Duration::from_millis(cfg.update_frequency);

        while !*game_finished.lock().unwrap() {
            let now = Instant::now();
            match get_chunka_chungus(&mut client) {
                Ok(snes_ram) => tx.send((Utc::now(), snes_ram))?,
                Err(_) => {
                    println!("Request failed, attempting to reconnect...");
                    if let Ok(connected_client) = connect(Arc::clone(&config)) {
                        client = connected_client;
                    }
                }
            }

            if cfg.manual_update {
                println!("Press enter to update...");
                stdin()
                    .read_line(&mut String::new())
                    .ok()
                    .expect("Failed to read line");
            } else {
                let elapsed = now.elapsed();
                if elapsed < update_freq {
                    sleep(update_freq - elapsed);
                }
                if cfg._verbosity > 0 {
                    println!("delta: {:?}", elapsed);
                }
            }
        }

        Ok(())
    });
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
    print: &mut StdoutPrinter,
    time_of_read: &DateTime<Utc>,
) -> anyhow::Result<()> {
    for check in checks {
        match &check.conditions {
            Some(conditions) => {
                if conditions
                    .iter()
                    .all(|c| match_condition(c, events, ram, ram_history))
                {
                    check.mark_as_checked(time_of_read);
                    print.location_check(check);

                    let location_check_event = EventEnum::LocationCheck(check.clone());
                    writer.serialize(Event::from(&location_check_event))?;
                    events.push(location_check_event);
                }
            }
            None => {
                let current_check_value =
                    ram.get_byte(check.sram_offset.unwrap_or_default() as usize);
                if ram_history.len() > 0
                    && (ram_history[ram_history.len() - 1]
                        .get_byte(check.sram_offset.unwrap_or_default() as usize)
                        != current_check_value)
                {
                    if current_check_value & check.sram_mask.unwrap_or_default() != 0
                        && !check.is_checked
                    {
                        check.mark_as_checked(time_of_read);
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
    print: &mut StdoutPrinter,
    time_of_read: &DateTime<Utc>,
) -> anyhow::Result<()> {
    for check in checks {
        let current_check_value = ram.get_byte(check.sram_offset.unwrap_or_default() as usize);

        if previous_values.len() > 0
            && (previous_values[previous_values.len() - 1]
                .get_byte(check.sram_offset.unwrap_or_default() as usize)
                != current_check_value)
        {
            if !check.is_progressive
                && current_check_value & check.sram_mask.unwrap_or_default() != 0
                && !check.is_checked
            {
                check.mark_as_checked(time_of_read);
                print.item_check(check);

                let item_event = EventEnum::ItemGet(check.clone());
                writer.serialize(Event::from(&item_event))?;
                events.push(item_event);
            } else if check.is_progressive && current_check_value > check.snes_value {
                check.progress_item(current_check_value, time_of_read);
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
    print: &mut StdoutPrinter,
    time_of_read: &DateTime<Utc>,
) -> anyhow::Result<()> {
    // Use events if one transition has been triggered.
    match events.latest_transition() {
        Some(previous_transition) => {
            if let Ok(mut current_tile) = Tile::try_from_ram(ram, &previous_transition) {
                if current_tile.name != previous_transition.name {
                    current_tile.time_transit(time_of_read);
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

fn check_for_events(
    ram: &SnesRam,
    previous_values: &mut VecDeque<SnesRam>,
    subscribed_events: &mut Vec<Check>,
    writer: &mut Writer<File>,
    events: &mut EventTracker,
    print: &mut StdoutPrinter,
    time_of_read: &DateTime<Utc>,
) -> anyhow::Result<bool> {
    for event in subscribed_events {
        let current_event_value = ram.get_byte(event.sram_offset.unwrap_or_default() as usize);
        match &event.conditions {
            Some(conditions) => {
                if (event.is_progressive || !event.is_checked)
                    && conditions
                        .iter()
                        .all(|condition| match_condition(condition, events, ram, previous_values))
                {
                    if !event.is_progressive {
                        event.mark_as_checked(time_of_read)
                    } else {
                        event.progress_item(current_event_value, time_of_read)
                    }
                    let occurred_event = EventEnum::Other(event.clone());
                    writer.serialize(Event::from(&occurred_event))?;
                    events.push(occurred_event);
                    print.event(event);
                    return Ok(event.id != 0 && event.id != 15);
                }
            }
            None => {
                if !event.is_progressive
                    && current_event_value & event.sram_mask.unwrap_or_default() != 0
                    && !event.is_checked
                {
                    event.mark_as_checked(time_of_read);
                    print.event(event);
                    let occurred_event = EventEnum::Other(event.clone());
                    writer.serialize(Event::from(&occurred_event))?;
                    events.push(occurred_event);
                } else if event.is_progressive && current_event_value > event.snes_value {
                    event.progress_item(current_event_value, time_of_read);
                    print.event(event);
                    let occurred_event = EventEnum::Other(event.clone());
                    writer.serialize(Event::from(&occurred_event))?;
                    events.push(occurred_event);
                    // Save & Quit and Reset will pause checks from occurring until player has gone in-game once more
                    return Ok(event.id != 0 && event.id != 15);
                }
            }
        }
    }

    Ok(true)
}

fn match_condition(
    condition: &Conditions,
    events: &EventTracker,
    ram: &SnesRam,
    previous_values: &mut VecDeque<SnesRam>,
) -> bool {
    match condition {
        Conditions::PreviousTile(condition) => {
            let previous_tile = &events
                .latest_transition()
                .expect("Transition should always exist");
            current_tile_condition_met(condition, previous_tile)
        }
        Conditions::Coordinates { coordinates } => coordinate_condition_met(&coordinates, ram),
        Conditions::Underworld => ram.indoors() == 1,
        Conditions::DungeonCounterIncreased { sram_offset } => {
            dungeon_counter_condition_met(previous_values, ram, sram_offset)
        }
        Conditions::ValueChanged { sram_offset } => {
            ram_value_change_condition_met(previous_values, ram, sram_offset)
        }
        Conditions::CurrentTile(condition) => {
            let current_tile = &events
                .latest_transition()
                .expect("Transition should always exist");
            current_tile_condition_met(condition, current_tile)
        }
        Conditions::Any { subconditions } => subconditions
            .iter()
            .any(|subcondition| match_condition(subcondition, events, ram, previous_values)),
        Conditions::PreviousEvent { id } => events
            .latest_other_event()
            .map(|e| e.id == *id)
            .unwrap_or(false),
        Conditions::BitWiseTrue {
            sram_offset: _,
            sram_mask: _,
        } => todo!(),
        Conditions::Not { subconditions } => subconditions
            .iter()
            .all(|subcondition| !match_condition(subcondition, events, ram, previous_values)),
        Conditions::ValueEq {
            sram_offset,
            sram_value,
        } => ram.get_byte(*sram_offset) == *sram_value,
        Conditions::CheckMade { id } => events.find_location_check(*id).is_some(),
        Conditions::PreviousValueEq {
            sram_offset,
            sram_value,
        } => {
            if previous_values.len() > 0 {
                let previous_ram = &previous_values[previous_values.len() - 1];
                previous_ram.get_byte(*sram_offset) == *sram_value
            } else {
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::tile::deserialize_transitions;

    use super::*;

    macro_rules! enforce_unique_ids {
        ($($name:ident: $checks:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let mut id_counter: HashMap<usize, usize> = HashMap::new();
                    for check in $checks.unwrap() {
                        *id_counter.entry(check.id).or_default() += 1;
                    }

                    id_counter.iter().for_each(|(id, occurences)| {
                        assert!(
                            *occurences == 1,
                            "ids have to be unique, yet id {} occurs {} times",
                            id,
                            occurences
                        )
                    })
                }
            )*
        };
    }

    enforce_unique_ids! {
        unique_event_ids: deserialize_event_checks(),
        unique_item_ids: deserialize_item_checks(),
        unique_location_ids: deserialize_location_checks(),
        unique_tile_ids: deserialize_transitions(),
    }
}
