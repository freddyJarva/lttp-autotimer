use crate::condition::Conditions;
use crate::event::{Event, EventEnum, EventLog, EventTracker};
use check::Check;

#[macro_use]
extern crate lazy_static;

use chrono::{DateTime, Utc};
use clap::ArgMatches;

use colored::Colorize;
use condition::Value::{CheckCount, EventCount, ItemCount, ValueOfAddress};
use condition::{
    coordinate_condition_met, current_tile_condition_met, dungeon_counter_condition_met,
    ram_value_change_condition_met,
};
use snes::SnesRam;
use tile::Tile;

use std::io::{stdin, Write};
use std::sync::{Arc, Mutex};

#[cfg(feature = "qusb")]
use std::sync::mpsc;

#[cfg(feature = "sni")]
use tokio::sync::mpsc;

use crate::check::{
    deserialize_event_checks, deserialize_item_checks, deserialize_location_checks,
};
use crate::output::StdoutPrinter;
#[cfg(feature = "qusb")]
use crate::qusb::{connect, fetch_metadata, init_allow_output, read_snes_ram};
use crate::snes::NamedAddresses;

use std::collections::VecDeque;
use std::fs::File;

use csv::Writer;

mod check;
mod event;
pub mod output;
#[cfg(feature = "qusb")]
mod qusb;
mod serde_lttp;
mod snes;

#[cfg(test)]
#[macro_use]
mod test_macros;

mod condition;
mod request;
#[cfg(feature = "sni")]
mod sni;
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

const TILE_INFO_CHUNK_SIZE: usize = 0x4c9;

const GAME_STATS_OFFSET: usize = 0xf418;
const GAME_STATS_SIZE: usize = 0xdf;

#[derive(Default, Clone)]
pub struct CliConfig {
    host: String,
    port: String,
    non_race_mode: bool,
    manual_update: bool,
    update_frequency: u64,
    _verbosity: u64,
}

#[cfg(feature = "qusb")]
pub fn connect_to_qusb(args: &ArgMatches) -> anyhow::Result<()> {
    let cli_config = CliConfig {
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
    };

    let (tx, rx) = mpsc::channel();

    let allow_output = Arc::new(Mutex::new(false));
    let game_finished = Arc::new(Mutex::new(false));

    let mut client = connect(cli_config.clone())?;
    init_allow_output(&mut client, cli_config.clone(), Arc::clone(&allow_output));
    let meta_data = match fetch_metadata(&mut client) {
        Ok(meta_data) => meta_data,
        Err(e) => {
            println!("Request for metadata failed, skipping. Cause: {:?}", e);
            None
        }
    };
    let mut print = StdoutPrinter::new(*allow_output.lock().unwrap());
    print.debug(format!(
        "{} metadata: {:?}",
        "Retrieved".green().bold(),
        meta_data
    ));
    read_snes_ram(tx, client, cli_config.clone(), Arc::clone(&game_finished));

    let mut ram_history: VecDeque<SnesRam> = VecDeque::new();

    let csv_name = Utc::now().format("%Y%m%d_%H%M%S.csv").to_string();
    let mut f = File::create(&csv_name)?;

    if let Some((permalink, meta_data)) = meta_data {
        match write_metadata_to_csv(&mut f, permalink, meta_data) {
            Ok(_) => print.debug(format!(
                "{} metadata to {}",
                "Wrote".green().bold(),
                csv_name
            )),
            Err(e) => println!("Failed fetching and/or writing metadata: {:?}", e),
        };
    }
    let mut writer = csv::WriterBuilder::new().from_writer(f);

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
    let mut items: Vec<Check> = deserialize_item_checks()?.into_iter().collect();

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

#[cfg(feature = "sni")]
#[tokio::main]
pub async fn connect_to_sni(args: &ArgMatches) -> anyhow::Result<()> {
    use crate::{
        request::fetch_metadata_for,
        sni::{
            api::{
                device_memory_client::DeviceMemoryClient, devices_response::Device, AddressSpace,
                DevicesRequest,
            },
            get_device, is_race_rom, read_rom_hash, read_snes_ram,
        },
    };

    let cli_config = CliConfig {
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
    };

    let game_finished = Arc::new(Mutex::new(false));

    println!("Connecting to sni");
    let sni_url = format!("ws://{}:{}", cli_config.host, cli_config.port);
    let connected_device = get_device(&sni_url).await?;

    let mut client = DeviceMemoryClient::connect(sni_url.to_string()).await?;
    let allow_output = match sni::is_race_rom(&connected_device, &mut client).await {
        Ok(is_race_rom) => !is_race_rom && cli_config.non_race_mode,
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
    };

    let meta_data = match sni::read_rom_hash(&connected_device, &mut client).await {
        Ok(hash) => match fetch_metadata_for(hash).await {
            Ok((permalink, meta)) => Some((permalink, meta.spoiler.meta)),
            Err(e) => {
                println!("Request for metadata failed, skipping. Cause: {:?}", e);
                None
            }
        },
        Err(e) => {
            println!("Reading rom hash failed. Cause: {:?}", e);
            None
        }
    };
    println!("{:?}", meta_data);
    let mut print = StdoutPrinter::new(allow_output);
    print.debug(format!(
        "{} metadata: {:?}",
        "Retrieved".green().bold(),
        meta_data
    ));

    let (tx, mut rx) = mpsc::channel(100);
    read_snes_ram(tx, client, connected_device, cli_config.clone()).await;

    let mut ram_history: VecDeque<SnesRam> = VecDeque::new();

    let csv_name = Utc::now().format("%Y%m%d_%H%M%S.csv").to_string();
    let mut f = File::create(&csv_name)?;

    if let Some((permalink, meta_data)) = meta_data {
        match write_metadata_to_csv(&mut f, permalink, meta_data) {
            Ok(_) => print.debug(format!(
                "{} metadata to {}",
                "Wrote".green().bold(),
                csv_name
            )),
            Err(e) => println!("Failed fetching and/or writing metadata: {:?}", e),
        };
    }
    let mut writer = csv::WriterBuilder::new().from_writer(f);

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
    let mut items: Vec<Check> = deserialize_item_checks()?.into_iter().collect();

    while let Some((time_of_read, snes_ram)) = rx.recv().await {
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

/// Metadata that will be written at the top of the csv
fn write_metadata_to_csv(
    f: &mut File,
    permalink: String,
    meta_data: request::MetaData,
) -> Result<(), anyhow::Error> {
    const NONE_STR: &'static str = "None";
    f.write_all(format!("# rom_build {}\n", meta_data.build).as_bytes())?;
    f.write_all(format!("# permalink {}\n", permalink).as_bytes())?;
    f.write_all(
        format!(
            "# name {}\n",
            meta_data.name.unwrap_or(NONE_STR.to_string())
        )
        .as_bytes(),
    )?;
    f.write_all(format!("# goal {}\n", meta_data.goal).as_bytes())?;
    f.write_all(format!("# mode {}\n", meta_data.mode).as_bytes())?;
    f.write_all(format!("# rom_mode {}\n", meta_data.rom_mode).as_bytes())?;
    f.write_all(format!("# logic {}\n", meta_data.logic).as_bytes())?;
    f.write_all(format!("# accessibility {}\n", meta_data.accessibility).as_bytes())?;
    f.write_all(format!("# weapons {}\n", meta_data.weapons).as_bytes())?;
    f.write_all(format!("# spoilers {}\n", meta_data.spoilers).as_bytes())?;
    f.write_all(format!("# tournament {}\n", meta_data.tournament).as_bytes())?;
    f.write_all(format!("# dungeon_items {}\n", meta_data.dungeon_items).as_bytes())?;
    f.write_all(format!("# item_pool {}\n", meta_data.item_pool).as_bytes())?;
    f.write_all(format!("# item_placement {}\n", meta_data.item_placement).as_bytes())?;
    f.write_all(format!("# item_functionality {}\n", meta_data.item_functionality).as_bytes())?;
    f.write_all(
        format!(
            "# enemizer_boss_shuffle {}\n",
            meta_data.enemizer_boss_shuffle
        )
        .as_bytes(),
    )?;
    f.write_all(
        format!(
            "# enemizer_enemy_damage {}\n",
            meta_data.enemizer_enemy_damage
        )
        .as_bytes(),
    )?;
    f.write_all(
        format!(
            "# enemizer_enemy_health {}\n",
            meta_data.enemizer_enemy_health
        )
        .as_bytes(),
    )?;
    f.write_all(
        format!(
            "# enemizer_enemy_shuffle {}\n",
            meta_data.enemizer_enemy_shuffle
        )
        .as_bytes(),
    )?;
    f.write_all(
        format!(
            "# enemizer_pot_shuffle {}\n",
            meta_data.enemizer_pot_shuffle
        )
        .as_bytes(),
    )?;
    f.write_all(
        format!(
            "# entry_crystals_ganon {}\n",
            meta_data.entry_crystals_ganon
        )
        .as_bytes(),
    )?;
    f.write_all(
        format!(
            "# entry_crystals_tower {}\n",
            meta_data.entry_crystals_tower
        )
        .as_bytes(),
    )?;
    f.write_all(format!("# allow_quickswap {}\n", meta_data.tournament).as_bytes())?;
    f.write_all(format!("# worlds {}\n", meta_data.worlds).as_bytes())?;
    f.write_all(format!("# world_id {}\n", meta_data.world_id).as_bytes())?;
    f.write_all(
        format!(
            "# notes {}\n",
            meta_data.notes.unwrap_or(NONE_STR.to_string())
        )
        .as_bytes(),
    )?;
    Ok(())
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

        match &check.conditions {
            Some(conditions) => {
                if (check.is_progressive || !check.is_checked)
                    && conditions
                        .iter()
                        .all(|condition| match_condition(condition, events, ram, previous_values))
                {
                    if !check.is_progressive {
                        check.mark_as_checked(time_of_read)
                    } else {
                        check.progress_item(current_check_value, time_of_read)
                    }
                    let occurred_check = EventEnum::ItemGet(check.clone());
                    writer.serialize(Event::from(&occurred_check))?;
                    events.push(occurred_check);
                    print.item_check(check);
                }
            }
            None => {
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
        Conditions::ValueGreaterThan { sram_offset, other } => match other {
            ValueOfAddress(other_address) => {
                ram.get_byte(*sram_offset) > ram.get_byte(*other_address)
            }
            CheckCount(_) => todo!(),
            ItemCount(id) => ram.get_byte(*sram_offset) > events.items_with_id(*id).len() as u8,
            EventCount(id) => ram.get_byte(*sram_offset) > events.others_with_id(*id).len() as u8,
        },
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
