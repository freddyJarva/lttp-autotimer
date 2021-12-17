use core::time;

use std::{
    borrow::Cow,
    io::stdin,
    net::TcpStream,
    sync::{mpsc, Arc, Mutex},
    thread::{self, sleep},
    time::Instant,
};

use chrono::{DateTime, Utc};
use colored::Colorize;
use serde::{Deserialize, Serialize};

use websocket::{sync::Client, ClientBuilder, Message, OwnedMessage};

use crate::{
    qusb,
    request::{fetch_metadata_for, MetaData},
    snes::SnesRam,
    CliConfig, COORDINATE_CHUNK_SIZE, COORDINATE_OFFSET, DUNKA_CHUNK_SIZE, DUNKA_START,
    GAME_STATS_OFFSET, GAME_STATS_SIZE, TILE_INFO_CHUNK_SIZE, VRAM_START,
};

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
        let message = &QusbRequestMessage::get_address(0x7fc0, 0x14);
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
