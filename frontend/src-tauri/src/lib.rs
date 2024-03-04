pub mod write;
pub mod state;

use std::collections::VecDeque;
use std::fs::File;
use std::io::{stdin, Write};
use std::sync::Arc;

use colored::Colorize;
use lttp_autotimer::check::{Check, deserialize_event_checks, deserialize_location_checks, deserialize_item_checks};
use lttp_autotimer::event::{EventTracker, EventLog};
use lttp_autotimer::output::StdoutPrinter;
use lttp_autotimer::snes::SnesRam;
use lttp_autotimer::{sni, time};
use chrono::{DateTime, Utc};
use lttp_autotimer::event::EventEnum::Command;

use lttp_autotimer::{
    check::{deserialize_actions, deserialize_commands},
    parse_ram::{
        check_for_actions, check_for_events, check_for_item_checks, check_for_location_checks,
        check_for_transitions, check_for_commands, check_for_segment_objective,
    },
    request,
    sni::{api::device_memory_client::DeviceMemoryClient, get_device, read_snes_ram}, event::{EventEnum, CommandState, EventCompactor}, time::{SingleRunStats, RunStatistics}, output::{format_duration, format_gold_duration, format_red_duration, TimeFormat},
};
use state::AppState;
use tauri::{AppHandle, Manager};
use tokio::sync::{mpsc, Mutex};

use crate::write::AppHandleWrapper;


pub async fn connect_to_sni(cli_config: lttp_autotimer::CliConfig, handle: AppHandle, state: Arc<Mutex<AppState>>) -> anyhow::Result<()> {

    let snes_handle = AppHandleWrapper::new(handle.clone());
    println!("Connecting to sni");
    let connected_device = get_device(&cli_config.sni_url()).await?;
    let mut client = DeviceMemoryClient::connect(cli_config.sni_url()).await?;
    let read_times = sni::check_read_times(&mut client, &connected_device).await?;

    let allow_output = match sni::is_race_rom(&connected_device, &mut client).await {
        Ok(is_race_rom) => !is_race_rom && cli_config.non_race_mode,
        Err(_) => {
            println!(
                "Wasn't able to tell if race rom or not, defaulting to showing output"
            );
            true
        }
    };
    if !allow_output {
        println!(
            "{}: no game info will be output in this window.\nNOTE: THIS TOOL IS NOT RACE LEGAL DESPITE VISUAL OUTPUT BEING TURNED OFF.",
            "Race mode activated".red(),
        )
    };

    let rom_hash = sni::read_rom_hash(&connected_device, &mut client).await?;
    let permalink = request::permalink_for(&rom_hash);
    let meta_data: Option<request::MetaData>;
    if cli_config.segment_run_mode {
        meta_data = None;
    } else {
        meta_data = match request::fetch_metadata_for(rom_hash).await {
            Ok(meta) => Some(meta.spoiler.meta),
            Err(e) => {
                println!("Request for metadata failed, skipping. Cause: {:?}", e);
                None
            }
        };
    }
    let mut print = StdoutPrinter::new(allow_output);
    print.debug(format!(
        "{} metadata: {:?}",
        "Retrieved".green().bold(),
        meta_data
    ));

    let (tx, mut rx) = mpsc::channel(200);
    read_snes_ram(tx, client, connected_device, cli_config.clone()).await;

    let mut ram_history: VecDeque<SnesRam> = VecDeque::new();

    let csv_name = Utc::now().format("%Y%m%d_%H%M%S.csv").to_string();
    let mut f = File::create(&csv_name)?;

    // let mut writer = csv::WriterBuilder::new().from_writer(f);
    let mut writer = snes_handle;

    let mut events = EventTracker::new();

    // Intro/start screen counts as not started. Having selected a spawn point counts as game started.
    // This is to ensure it only checks for events - especially transitions - while in-game.
    let mut game_started = false;

    // segment recorder states
    let mut command_state: CommandState = CommandState::None;

    let mut subscribed_events: Vec<Check> = deserialize_event_checks()?;
    let mut locations: Vec<Check> = deserialize_location_checks()?
        .into_iter()
        // 0 offset checks without conditions hasn't been given a proper value in checks.json yet
        .filter(|check| check.sram_offset.unwrap_or_default() != 0 || check.conditions.is_some())
        .collect();
    let mut items: Vec<Check> = deserialize_item_checks()?.into_iter().collect();
    let mut actions: Vec<Check> = deserialize_actions()?.into_iter().collect();

    let mut last_event_log_clear = Utc::now();

    while let Some((time_of_read, snes_ram)) = rx.recv().await {
        if !game_started {
            game_started = snes_ram.game_has_started();
            continue;
        } 
        // command mode

        {
            let mut state_unlocked = state.lock().await;
            match state_unlocked.command {
                CommandState::ClearEventLog => {
                    // check delta
                    let delta = Utc::now() - last_event_log_clear;
                    if delta.num_seconds() > 1 {
                        events = EventTracker::new();
                        locations = deserialize_location_checks()?;
                        subscribed_events = deserialize_event_checks()?
                            .into_iter()
                            .map(|e| {
                                if e.is_checked {
                                    println!("Somehow {} is already checked, even though resetting", e.name);
                                }
                                e
                            })
                            .collect();
                        items = deserialize_item_checks()?.into_iter().collect();
                        actions = deserialize_actions()?.into_iter().collect();
                        last_event_log_clear = Utc::now();
                    } else {
                        println!("Not clearing event log, too soon since last clear");
                    }

                    state_unlocked.command = CommandState::None;
                }
                _ => ()
            }
        }

        let should_print = !(matches!(command_state, CommandState::RunStarted(_))
             || matches!(command_state, CommandState::RunFinished));
        // checks
        game_started = check_for_events(
            &snes_ram,
            &mut ram_history,
            &mut subscribed_events,
            &mut writer,
            &mut events,
            &mut print,
            &time_of_read,
            should_print || cli_config._verbosity > 0
        )?;
        if game_started {
            if cli_config._verbosity > 1 {
                check_for_actions(
                    &snes_ram,
                    &mut ram_history,
                    &mut actions,
                    &mut writer,
                    &mut events,
                    &mut print,
                    &time_of_read,
                    should_print
                )?;
            }
            check_for_transitions(
                &snes_ram,
                &mut writer,
                &mut events,
                &mut print,
                &time_of_read,
                should_print || cli_config._verbosity > 0
            )?;
            check_for_location_checks(
                &snes_ram,
                &mut ram_history,
                &mut locations,
                &mut writer,
                &mut events,
                &mut print,
                &time_of_read,
                should_print || cli_config._verbosity > 0
            )?;
            check_for_item_checks(
                &snes_ram,
                &mut ram_history,
                &mut items,
                &mut writer,
                &mut events,
                &mut print,
                &time_of_read,
                should_print || cli_config._verbosity > 0
            )?;
        }
        ram_history.push_back(snes_ram);

        // Only keep the last few responses to decrease memory usage
        if ram_history.len() > 60 {
            ram_history.pop_front();
        }

        // checks to see if Ganon was just defeated.
        // basically, don't check for events in end credits
        if events
            .latest_other_event()
            .map(|event| event.id == 5)
            .unwrap_or(false)
            || events
                .latest_transition()
                .map(|tile| tile.id == 556)
                .unwrap_or(false)
        {
            game_started = false;
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
    meta_data: Option<request::MetaData>,
    mut read_times: Vec<u128>,
) -> Result<(), anyhow::Error> {
    const NONE_STR: &'static str = "None";
    f.write_all(format!("# logger_version {}\n", env!("CARGO_PKG_VERSION")).as_bytes())?;
    f.write_all(format!("# permalink {}\n", permalink).as_bytes())?;
    f.write_all(
        format!(
            "# read_time_ms_avg {}\n",
            read_times.iter().sum::<u128>() as usize / read_times.len()
        )
        .as_bytes(),
    )?;
    read_times.sort();
    f.write_all(format!("# read_time_ms_mean {}\n", read_times[read_times.len() / 2]).as_bytes())?;

    if let Some(meta_data) = meta_data {
        f.write_all(format!("# rom_build {}\n", meta_data.build).as_bytes())?;
        f.write_all(
            format!(
                "# name {}\n",
                meta_data
                    .name
                    .unwrap_or(NONE_STR.to_string())
                    .replace("\n", " ")
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
                meta_data
                    .notes
                    .unwrap_or(NONE_STR.to_string())
                    .replace("\n", " ")
            )
            .as_bytes(),
        )?;
    }
    Ok(())
}
