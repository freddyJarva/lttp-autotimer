use serde::{Deserialize, Serialize};

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
}
