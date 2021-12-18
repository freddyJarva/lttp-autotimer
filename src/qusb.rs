use core::time;

use std::{
    borrow::Cow,
    collections::VecDeque,
    fs::File,
    io::stdin,
    net::TcpStream,
    sync::{mpsc, Arc, Mutex},
    thread::{self, sleep},
    time::Instant,
};

use chrono::{DateTime, Utc};
use clap::ArgMatches;
use colored::Colorize;
use serde::{Deserialize, Serialize};

use websocket::{sync::Client, ClientBuilder, Message, OwnedMessage};

use crate::{
    check::{
        deserialize_event_checks, deserialize_item_checks, deserialize_location_checks, Check,
    },
    event::{EventLog, EventTracker},
    output::StdoutPrinter,
    parse_ram::{
        check_for_events, check_for_item_checks, check_for_location_checks, check_for_transitions,
    },
    qusb,
    request::{fetch_metadata_for, MetaData},
    snes::SnesRam,
    write_metadata_to_csv, CliConfig, VRAM_START,
};

#[derive(Copy, Clone)]
pub enum Address {
    RaceRom = 0x180213,
    RomHash = 0x7fc0,
    RomHashSize = 0x14,
    TileInfoChunk = 0xf50000,
    TileInfoSize = 0x4c9,
    DunkaChunka = 0xf5f021,
    DunkaChunkaSize = 0x3f1,
    GameStats = 0xf5f418,
    GameStatsSize = 0xdf,
    Coordinates = 0xf5c184,
    CoordinatesSize = 0x4,
}

impl Address {
    pub fn address(&self) -> usize {
        *self as usize
    }

    pub fn offset(&self) -> usize {
        let address = self.address();
        if address <= VRAM_START as usize {
            0
        } else {
            *self as usize - VRAM_START as usize
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct QusbResponseMessage {
    #[serde(rename = "Results")]
    pub results: Vec<String>,
}

#[derive(Serialize, Debug)]
pub struct QusbRequestMessage {
    #[serde(rename = "Opcode")]
    pub op_code: String,
    #[serde(rename = "Space")]
    pub space: String,
    #[serde(rename = "Operands")]
    pub operands: Option<Vec<String>>,
}

impl QusbRequestMessage {
    /// Convenience function for creating a device list message, as its values are static
    pub fn device_list() -> Self {
        QusbRequestMessage {
            op_code: "DeviceList".to_string(),
            space: "SNES".to_string(),
            operands: None,
        }
    }

    pub fn attach_to<S: AsRef<str>>(device: S) -> Self {
        QusbRequestMessage {
            op_code: "Attach".to_string(),
            space: "SNES".to_string(),
            operands: Some(vec![device.as_ref().to_string()]),
        }
    }

    pub fn device_info<S: AsRef<str>>(device: S) -> Self {
        QusbRequestMessage {
            op_code: "Info".to_string(),
            space: "SNES".to_string(),
            operands: Some(vec![device.as_ref().to_string()]),
        }
    }

    pub fn get_address(address: u32, size: usize) -> Self {
        let operands = Some(vec![format!("{:X}", address), format!("{:X}", size)]);
        QusbRequestMessage {
            op_code: "GetAddress".to_string(),
            space: "SNES".to_string(),
            operands,
        }
    }
}

pub fn start(args: &ArgMatches) -> anyhow::Result<()> {
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

pub fn attempt_qusb_connection(
    client: &mut websocket::sync::Client<std::net::TcpStream>,
) -> Result<bool, anyhow::Error> {
    let qusb_message = serde_json::to_vec(&QusbRequestMessage::device_list())?;
    let message = Message {
        opcode: websocket::message::Type::Text,
        cd_status_code: None,
        payload: Cow::Owned(qusb_message),
    };
    let mut connected = false;
    client.send_message(&message)?;
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
                connected = true;
                println!("{}", "Attached!".green().bold());
            }
            None => (),
        }
    }

    Ok(connected)
}

pub fn init_allow_output(
    client: &mut Client<TcpStream>,
    config: CliConfig,

    allow_output_rx: Arc<Mutex<bool>>,
) {
    *allow_output_rx.lock().unwrap() = match is_race_rom(client) {
        Ok(race_rom) => {
            if race_rom {
                false
            } else {
                config.non_race_mode
            }
        }
        Err(_) => {
            println!(
                "Wasn't able to tell if race rom or not, defaulting to not allowing any event output"
            );
            false
        }
    };
    if !*allow_output_rx.lock().unwrap() {
        println!(
            "{}: no game info will be output in this window.\nNOTE: THIS TOOL IS NOT RACE LEGAL DESPITE VISUAL OUTPUT BEING TURNED OFF.",
            "Race mode activated".red(),
        )
    }
}

pub fn fetch_metadata(
    client: &mut Client<TcpStream>,
) -> Result<Option<(String, MetaData)>, anyhow::Error> {
    let rom_hash = read_rom_hash(client)?;
    match rom_hash {
        Some(rom_hash) => {
            println!("{} seed {}", "Playing".green().bold(), rom_hash.cyan());
            let (permalink, json) = fetch_metadata_for(rom_hash)?;
            Ok(Some((permalink, json.spoiler.meta)))
        }
        None => {
            println!("Failed to read rom hash, skipping requesting metadata");
            Ok(None)
        }
    }
}

pub fn connect(
    config: CliConfig,
) -> Result<websocket::sync::Client<std::net::TcpStream>, anyhow::Error> {
    let mut client =
        ClientBuilder::new(&format!("ws://{}:{}", config.host, config.port))?.connect_insecure()?;
    while !attempt_qusb_connection(&mut client)? {
        sleep(time::Duration::from_millis(2000));
    }
    Ok(client)
}

pub fn read_rom_hash(
    client: &mut websocket::sync::Client<std::net::TcpStream>,
) -> anyhow::Result<Option<String>> {
    loop {
        let message = &QusbRequestMessage::get_address(
            Address::RomHash as u32,
            Address::RomHashSize as usize,
        );
        let message = Message {
            opcode: websocket::message::Type::Text,
            cd_status_code: None,
            payload: Cow::Owned(serde_json::to_vec(message)?),
        };
        client.send_message(&message)?;
        let response = client.recv_message()?;
        if let OwnedMessage::Binary(res) = response {
            let s = std::str::from_utf8(&res)?;
            return Ok(s.split_ascii_whitespace().nth(1).map(|s| s.to_string()));
        };
    }
}

pub fn is_race_rom(
    client: &mut websocket::sync::Client<std::net::TcpStream>,
) -> anyhow::Result<bool> {
    loop {
        let message = &QusbRequestMessage::get_address(Address::RaceRom as u32, 1);
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

pub fn read_snes_ram(
    tx: mpsc::Sender<(DateTime<Utc>, SnesRam)>,
    mut client: websocket::sync::Client<std::net::TcpStream>,
    config: CliConfig,
    game_finished: Arc<Mutex<bool>>,
) {
    thread::spawn(move || -> anyhow::Result<()> {
        let update_freq = time::Duration::from_millis(config.update_frequency);

        while !*game_finished.lock().unwrap() {
            let now = Instant::now();
            match get_chunka_chungus(&mut client) {
                Ok(snes_ram) => tx.send((Utc::now(), snes_ram))?,
                Err(_) => {
                    println!("Request failed, attempting to reconnect...");
                    client.shutdown()?;
                    if let Ok(connected_client) = connect(config.clone()) {
                        client = connected_client;
                    } else {
                        println!("Failed")
                    }
                }
            }

            if config.manual_update {
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
                if config._verbosity > 0 {
                    println!("delta: {:?}", elapsed);
                }
            }
        }

        Ok(())
    });
}

// since we can't choose multiple addresses in a single request, we instead fetch a larger chunk of data from given address and forward
// so we don't have to make multiple requests
pub fn get_chunka_chungus(
    client: &mut websocket::sync::Client<std::net::TcpStream>,
) -> anyhow::Result<SnesRam> {
    let tile_info_message = &QusbRequestMessage::get_address(
        Address::TileInfoChunk as u32,
        Address::TileInfoSize as usize,
    );
    let dunka_chunka_message = &QusbRequestMessage::get_address(
        Address::DunkaChunka as u32,
        Address::DunkaChunkaSize as usize,
    );
    let coordinate_message = &QusbRequestMessage::get_address(
        // VRAM_START + COORDINATE_OFFSET as u32,
        Address::Coordinates as u32,
        Address::CoordinatesSize as usize,
    );
    let game_stats_message = &QusbRequestMessage::get_address(
        // VRAM_START + GAME_STATS_OFFSET as u32,
        // GAME_STATS_SIZE,
        Address::GameStats as u32,
        Address::GameStatsSize as usize,
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
