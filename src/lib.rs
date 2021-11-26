use crate::serde_lttp::hex_serialize_option;
use check::Check;
use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use serde::Serialize;
use transition::Transition;

pub mod check;
pub mod output;
pub mod qusb;
mod serde_lttp;
pub mod snes;
pub mod transition;

/// Snes memory address
pub const VRAM_START: u32 = 0xf50000;
pub const SAVE_DATA_OFFSET: usize = 0xF000;
pub const SAVEDATA_START: u32 = VRAM_START + SAVE_DATA_OFFSET as u32;
/// I'm too lazy to manually translate dunka's values, so I'll just use this instead to read from the correct memory address
pub const DUNKA_VRAM_READ_OFFSET: u32 = SAVEDATA_START + 0x280;
pub const DUNKA_VRAM_READ_SIZE: u32 = 0x280;

/// Address keeping track of current overworld tile, remains at previous value when entering non-ow tile
pub const ADDRESS_OW_SLOT_INDEX: u32 = 0x7E040A;
/// Address keeping track of latest entrance transition, i.e. walking in or out of house/dungeon/etc
pub const ADDRESS_ENTRANCE_ID: u32 = 0x7E010E;
/// Address that's `1` if Link is inside, `0` if outside;
pub const ADDRESS_IS_INSIDE: u32 = 0x7E001B;

#[derive(Serialize, Debug)]
pub struct Event {
    #[serde(with = "ts_milliseconds")]
    timestamp: DateTime<Utc>,
    indoors: Option<bool>,
    #[serde(serialize_with = "hex_serialize_option")]
    to: Option<u16>,
    location_id: Option<String>,
    item_id: Option<String>,
}

impl From<&Transition> for Event {
    fn from(transition: &Transition) -> Self {
        Event {
            timestamp: transition.timestamp,
            indoors: Some(transition.indoors),
            to: Some(transition.to),
            location_id: None,
            item_id: None,
        }
    }
}

impl From<&Check> for Event {
    fn from(check: &Check) -> Self {
        Event {
            timestamp: check
                .time_of_check
                .expect("Found check missing timestamp when serializing"),
            indoors: None,
            to: None,
            location_id: Some(check.name.to_string()),
            item_id: match &check.item {
                Some(item) => Some(item.to_string()),
                None => None,
            },
        }
    }
}

impl From<&mut Check> for Event {
    fn from(check: &mut Check) -> Self {
        Event {
            timestamp: check
                .time_of_check
                .expect("Found check missing timestamp when serializing"),
            indoors: None,
            to: None,
            location_id: Some(check.name.to_string()),
            item_id: match &check.item {
                Some(item) => Some(item.to_string()),
                None => None,
            },
        }
    }
}
