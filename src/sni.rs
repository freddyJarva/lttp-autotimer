use chrono::{DateTime, Utc};
use colored::Colorize;
use core::time;
use std::io::stdin;

use std::thread;
use std::time::Instant;
use tokio::sync::mpsc;
use tonic::transport::Channel;

use anyhow::{anyhow, Result};
use api::devices_client::DevicesClient;
use api::{DevicesRequest, DevicesResponse};
use tonic::Response;

use self::api::device_memory_client::DeviceMemoryClient;
use self::api::devices_response::Device;
use self::api::{
    AddressSpace, MemoryMapping, MultiReadMemoryRequest, ReadMemoryRequest, SingleReadMemoryRequest,
};

use crate::snes::SnesRam;
use crate::CliConfig;

pub enum Address {
    RaceRom = 0x180213,
    RomHash = 0x007fc0,
    RomHashSize = 0x14,
    TileInfoChunk = 0xf50000,
    TileInfoSize = 0x4c9,
    DunkaChunka = 0xf5f021,
    // DunkaChunkaSize = 0x3f1,
    DunkaChunkaSize = 0x4f7,
    // GameStats = 0xf5f418,
    // GameStatsSize = 0xdf,
    Coordinates = 0xf5c184,
    CoordinatesSize = 0x4,
}

pub mod api {
    tonic::include_proto!("_");
}

pub async fn list_devices(url: &str) -> Result<Response<DevicesResponse>> {
    let mut client = DevicesClient::connect(url.to_string()).await?;
    let devices = client
        .list_devices(DevicesRequest {
            ..Default::default()
        })
        .await?;
    Ok(devices)
}

pub async fn get_device(url: &str) -> Result<Device> {
    loop {
        let devices = list_devices(url).await?;
        println!("Devices: {:?}", devices.get_ref().devices);
        if devices.get_ref().devices.len() > 0 {
            let device = devices.get_ref().devices[0].clone();
            println!(
                "{} to the first option in devices: {}",
                "Attaching".green().bold(),
                &device.display_name
            );
            return Ok(device);
        } else {
            thread::sleep(time::Duration::from_secs(2))
        }
    }
}

pub async fn is_race_rom(
    device: &Device,
    client: &mut DeviceMemoryClient<Channel>,
) -> Result<bool> {
    let response = client
        .single_read(SingleReadMemoryRequest {
            uri: device.uri.to_string(),
            request: Some(ReadMemoryRequest {
                request_address: Address::RaceRom as u32,
                request_address_space: AddressSpace::FxPakPro as i32,
                request_memory_mapping: MemoryMapping::LoRom as i32,
                size: 1,
            }),
        })
        .await?;
    println!("race_rom: {:?}", &response.get_ref());
    match &response.get_ref().response {
        Some(r) => Ok(r.data[0] == 1),
        None => Err(anyhow!(
            "Failed to read race rom address {:X}",
            Address::RaceRom as u32
        )),
    }
}

pub async fn read_rom_hash(
    device: &Device,
    client: &mut DeviceMemoryClient<Channel>,
) -> Result<String> {
    let response = client
        .single_read(SingleReadMemoryRequest {
            uri: device.uri.clone(),
            request: Some(ReadMemoryRequest {
                request_address: Address::RomHash as u32,
                request_address_space: AddressSpace::FxPakPro as i32,
                request_memory_mapping: MemoryMapping::LoRom as i32,
                size: Address::RomHashSize as u32,
            }),
        })
        .await?;
    match &response.get_ref().response {
        Some(r) => {
            let s = std::str::from_utf8(&r.data)?;
            println!("{}", s);
            Ok(s.split_ascii_whitespace()
                .nth(1)
                .map(|s| s.to_string())
                .ok_or(anyhow!("Failed to parse rom hash from string {}", s))?)
        }
        None => Err(anyhow!(
            "Failed to read rom hash address {:X}",
            Address::RomHash as u32
        )),
    }
}

pub async fn read_snes_ram(
    tx: mpsc::Sender<(DateTime<Utc>, SnesRam)>,
    mut client: DeviceMemoryClient<Channel>,
    device: Device,
    config: CliConfig,
) {
    tokio::spawn(async move {
        let update_freq = time::Duration::from_millis(config.update_frequency);

        loop {
            let now = Instant::now();
            match get_chunka_chungus(&device, &mut client).await {
                Ok(snes_ram) => match tx.send((Utc::now(), snes_ram)).await {
                    Ok(_) => (),
                    Err(_) => (),
                },
                Err(_) => {

                    // println!("Request failed, attempting to reconnect...");
                    // client.shutdown()?;
                    // if let Ok(connected_client) = connect(config.clone()) {
                    //     client = connected_client;
                    // } else {
                    //     println!("Failed")
                    // }
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
                    thread::sleep(update_freq - elapsed);
                }
                if config._verbosity > 0 {
                    println!("delta: {:?}", elapsed);
                }
            }
        }
    });
}

pub async fn get_chunka_chungus(
    device: &Device,
    client: &mut DeviceMemoryClient<Channel>,
) -> Result<SnesRam> {
    let multi_message = MultiReadMemoryRequest {
        uri: device.uri.to_string(),
        requests: vec![
            ReadMemoryRequest {
                request_address: Address::TileInfoChunk as u32,
                request_address_space: AddressSpace::FxPakPro as i32,
                request_memory_mapping: MemoryMapping::LoRom as i32,
                size: Address::TileInfoSize as u32,
            },
            ReadMemoryRequest {
                request_address: Address::DunkaChunka as u32,
                request_address_space: AddressSpace::FxPakPro as i32,
                request_memory_mapping: MemoryMapping::LoRom as i32,
                size: Address::DunkaChunkaSize as u32,
            },
            // ReadMemoryRequest {
            //     request_address: Address::GameStats as u32,
            //     request_address_space: AddressSpace::FxPakPro as i32,
            //     request_memory_mapping: MemoryMapping::LoRom as i32,
            //     size: Address::GameStatsSize as u32,
            // },
            ReadMemoryRequest {
                request_address: Address::Coordinates as u32,
                request_address_space: AddressSpace::FxPakPro as i32,
                request_memory_mapping: MemoryMapping::LoRom as i32,
                size: Address::CoordinatesSize as u32,
            },
        ],
    };

    let now = Instant::now();
    let mut response = client.multi_read(multi_message).await?;
    println!("{:?}", now.elapsed());

    let snes_ram = SnesRam::from(&response.get_mut().responses);

    Ok(snes_ram)
}
