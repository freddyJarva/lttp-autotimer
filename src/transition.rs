use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde::Serializer;

use crate::snes::NamedAddresses;

pub fn overworld_transition(previous_res: &Vec<u8>, response: &Vec<u8>) -> bool {
    previous_res.overworld_tile() != response.overworld_tile()
}

pub fn entrance_transition(previous_res: &Vec<u8>, response: &Vec<u8>) -> bool {
    previous_res.indoors() != response.indoors()
}

fn hex_serialize<S>(x: &u16, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(format!("{:X}", x).as_ref())
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
