use crate::serde_lttp::hex_serialize;
use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::snes::NamedAddresses;

pub fn overworld_transition<T: AsRef<[u8]>, U: AsRef<[u8]>>(previous_res: T, response: U) -> bool {
    previous_res.as_ref().overworld_tile() != response.as_ref().overworld_tile()
}

pub fn entrance_transition<T: AsRef<[u8]>, U: AsRef<[u8]>>(previous_res: T, response: U) -> bool {
    previous_res.as_ref().indoors() != response.as_ref().indoors()
}

#[derive(Serialize, Debug)]
pub struct Transition {
    #[serde(with = "ts_milliseconds")]
    pub timestamp: DateTime<Utc>,
    pub indoors: bool,
    #[serde(serialize_with = "hex_serialize")]
    pub to: u16,
}

impl Transition {
    pub fn new(to: u16, indoors: bool) -> Self {
        Transition {
            timestamp: Utc::now(),
            indoors,
            to,
        }
    }
}
