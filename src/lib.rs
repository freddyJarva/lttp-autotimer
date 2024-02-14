use crate::event::{EventLog, EventTracker};
use check::Check;

#[macro_use]
extern crate lazy_static;

use chrono::Utc;
use clap::ArgMatches;

use colored::Colorize;
use snes::SnesRam;

use std::io::{stdin, Write};

#[cfg(feature = "sni")]
use tokio::sync::mpsc;

use crate::check::{
    deserialize_event_checks, deserialize_item_checks, deserialize_location_checks,
};
use crate::output::StdoutPrinter;

use std::collections::VecDeque;
use std::fs::File;

mod check;
mod condition;
mod event;
pub mod output;
mod request;
mod serde_lttp;
mod snes;
mod tile;

#[cfg(feature = "sni")]
mod sni;

#[cfg(test)]
#[macro_use]
mod test_macros;
mod parse_ram;
mod write;
mod time;

/// Snes memory address
pub const VRAM_START: u32 = 0xf50000;

#[derive(Default, Clone)]
pub struct CliConfig {
    host: String,
    port: String,
    non_race_mode: bool,
    manual_update: bool,
    update_frequency: u64,
    _verbosity: u64,
    segment_run_mode: bool,
}

impl CliConfig {
    pub fn sni_url(&self) -> String {
        format!("ws://{}:{}", self.host, self.port)
    }
}


#[cfg(feature = "sni")]
#[tokio::main]
pub async fn connect_to_sni(args: &ArgMatches) -> anyhow::Result<()> {
    use chrono::{DateTime, Duration};
    use event::EventEnum::Command;

    use crate::{
        check::{deserialize_actions, deserialize_commands},
        parse_ram::{
            check_for_actions, check_for_events, check_for_item_checks, check_for_location_checks,
            check_for_transitions, check_for_commands, check_for_segment_run_start,
        },
        request::fetch_metadata_for,
        sni::{api::device_memory_client::DeviceMemoryClient, get_device, read_snes_ram}, event::{EventEnum, CommandState}, time::{SequenceStatistics, SingleRunStats, RunStatistics}, output::{format_duration, format_gold_duration, format_red_duration},
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
        segment_run_mode: args.is_present("Segment run mode")
    };

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
        meta_data = match fetch_metadata_for(rom_hash).await {
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

    match write_metadata_to_csv(&mut f, permalink, meta_data, read_times) {
        Ok(_) => print.debug(format!(
            "{} metadata to {}",
            "Wrote".green().bold(),
            csv_name
        )),
        Err(e) => println!("Failed fetching and/or writing metadata: {:?}", e),
    };

    let mut writer = csv::WriterBuilder::new().from_writer(f);

    let mut events = EventTracker::new();

    // Intro/start screen counts as not started. Having selected a spawn point counts as game started.
    // This is to ensure it only checks for events - especially transitions - while in-game.
    let mut game_started = false;

    // segment recorder states
    let mut command_state: CommandState = CommandState::None;
    let mut segment_objectives: Vec<EventEnum> = vec![];
    let mut finished_objectives: Vec<(EventEnum, DateTime<Utc>)> = vec![];
    let mut finished_runs: Vec<Vec<(EventEnum, DateTime<Utc>)>> = vec![];
    let mut segment_times: Vec<Duration> = vec![];

    let mut subscribed_events: Vec<Check> = deserialize_event_checks()?;
    let mut locations: Vec<Check> = deserialize_location_checks()?
        .into_iter()
        // 0 offset checks without conditions hasn't been given a proper value in checks.json yet
        .filter(|check| check.sram_offset.unwrap_or_default() != 0 || check.conditions.is_some())
        .collect();
    let mut items: Vec<Check> = deserialize_item_checks()?.into_iter().collect();
    let mut actions: Vec<Check> = deserialize_actions()?.into_iter().collect();
    let mut commands: Vec<Check> = deserialize_commands()?.into_iter().collect();

    while let Some((time_of_read, snes_ram)) = rx.recv().await {
        if !game_started {
            game_started = snes_ram.game_has_started();
        } else {
            // command mode
            if cli_config.segment_run_mode {
                let input_command = check_for_commands(
                    &snes_ram,
                    &mut ram_history,
                    &mut commands,
                    &mut writer,
                    &mut events,
                    &mut print,
                    &time_of_read,
                )?;
                let input_cmd = input_command.unwrap_or(Check::new(0));
                match command_state {
                    CommandState::None => {
                        match &input_cmd.id {
                            1 => {
                                command_state = CommandState::RecordingInProgress(input_cmd);
                            },
                            _ => ()
                        }
                    },
                    CommandState::RecordingInProgress(ref start_check) => {
                        match &input_cmd.id {
                            1 => {
                                command_state = CommandState::RecordingInProgress(input_cmd);
                            },
                            2 => {
                                if start_check.id == 1 {
                                    segment_objectives = events.objectives_between(Command(start_check.clone()), Some(Command(input_cmd)));
                                    println!("Segments: {:?}", segment_objectives);
                                    command_state = CommandState::SegmentRecorded;
                                }
                            },
                            _ => ()
                        }
                    },
                    CommandState::SegmentRecorded if segment_objectives.len() > 0 => {
                        match &input_cmd.id {
                            1 => {
                                command_state = CommandState::RecordingInProgress(input_cmd);
                            },
                            _ => {
                                let objective = segment_objectives.first().expect("Length should be > 0 here").clone().to_owned();

                                if check_for_segment_run_start(
                                    &snes_ram,
                                    &mut ram_history,
                                    &objective,
                                    &mut events
                                )? {
                                    finished_objectives.push((objective, time_of_read.clone()));
                                    println!("Run start");
                                    command_state = CommandState::RunStarted(1)
                                }
                            }
                        }
                    },
                    CommandState::RunStarted(current_objective_idx) => {
                        match &input_cmd.id {
                            1 => {
                                finished_objectives = vec![];
                                command_state = CommandState::RecordingInProgress(input_cmd);
                            },
                            2 => {
                                println!("Stopped current run");
                                finished_objectives = vec![];
                                command_state = CommandState::SegmentRecorded;
                            },
                            _ => {
                                if segment_objectives.len() <= current_objective_idx {
                                    // Run finished
                                    command_state = CommandState::RunFinished;
                                } else {
                                    let objective = &segment_objectives[current_objective_idx];
                                    if check_for_segment_run_start(
                                        &snes_ram,
                                        &mut ram_history,
                                        &objective,
                                        &mut events
                                    )? {
                                        finished_objectives.push((objective.clone(), time_of_read.clone()));
                                        let otime = finished_objectives.objective_duration(current_objective_idx).unwrap();
                                        match finished_runs.objective_time_verdict(current_objective_idx, &otime) {
                                            time::TimeVerdict::Bad => println!("{}/{} - {}: {}",current_objective_idx, segment_objectives.len() - 1, objective.name(), format_red_duration(otime)),
                                            time::TimeVerdict::Ok => println!("{}/{} - {}: {}",current_objective_idx, segment_objectives.len() - 1, objective.name(), format_duration(otime)),
                                            time::TimeVerdict::Best => println!("{}/{} - {}: {}",current_objective_idx, segment_objectives.len() - 1, objective.name(), format_gold_duration(otime)),
                                        }
                                        command_state = CommandState::RunStarted(current_objective_idx + 1)
                                    }
                                }
                            }
                        }
                    },
                    CommandState::SegmentRecorded => {
                        command_state = CommandState::None;
                    },
                    CommandState::RunFinished => {
                        print.segment_finish(&finished_objectives);
                        // TODO: print to csv, calculate averages etc.
                        let start = finished_objectives[0].1.clone();
                        let end = finished_objectives[finished_objectives.len() - 1].1.clone();
                        let time = end - start;
                        segment_times.push(time);
                        println!("avg: {}", output::format_duration(segment_times.avg()));
                        let n_avg = 5;
                        println!("rolling_avg ({}): {}", n_avg, output::format_duration(segment_times.rolling_avg(n_avg)));

                        finished_runs.push(finished_objectives);
                        finished_objectives = vec![];
                        command_state = CommandState::SegmentRecorded;
                        events = EventTracker::new();
                    }
                }
            }
            if !(matches!(command_state, CommandState::RunStarted(_))
                 || matches!(command_state, CommandState::RunFinished)) {
                // checks
                game_started = check_for_events(
                    &snes_ram,
                    &mut ram_history,
                    &mut subscribed_events,
                    &mut writer,
                    &mut events,
                    &mut print,
                    &time_of_read,
                    false,
                )?;
                if game_started {
                    if cli_config._verbosity > 0 {
                        check_for_actions(
                            &snes_ram,
                            &mut ram_history,
                            &mut actions,
                            &mut writer,
                            &mut events,
                            &mut print,
                            &time_of_read,
                        )?;
                    }
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
                        false,
                    )?;
                    check_for_item_checks(
                        &snes_ram,
                        &mut ram_history,
                        &mut items,
                        &mut writer,
                        &mut events,
                        &mut print,
                        &time_of_read,
                        false,
                    )?;
                }
            }
            ram_history.push_back(snes_ram);
        }

        // Only keep the last few responses to decrease memory usage
        if ram_history.len() > 60 {
            ram_history.pop_front();
        }

        writer.flush()?;

        // check if recording segment

        if events
            .latest_other_event()
            .map(|event| event.id == 5)
            .unwrap_or(false)
            || events
                .latest_transition()
                .map(|tile| tile.id == 556)
                .unwrap_or(false)
        {
            // Game is finished
            break;
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{check::deserialize_actions, tile::deserialize_transitions, check::deserialize_commands};

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
        unique_action_ids: deserialize_actions(),
        unique_commands_ids: deserialize_commands(),
    }
}
