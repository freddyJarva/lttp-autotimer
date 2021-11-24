use std::borrow::Cow;

use colored::Colorize;
use serde::{Deserialize, Serialize};
use websocket::{Message, OwnedMessage};

use crate::qusb;

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
