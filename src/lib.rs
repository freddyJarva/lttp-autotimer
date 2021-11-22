use chrono::serde::ts_milliseconds;
use chrono::DateTime;
use chrono::Utc;
use serde::Serialize;

use crate::request::Response;
use crate::request::MESSAGE_TERMINATOR;

pub mod read;
pub mod request;

/// Snes memory address
pub const VRAM_START: u32 = 0x7E0000;
pub const VRAM_START_U8: &[u8; 8] = b"0x7E0000";
/// Snes memory address
pub const VRAM_END: u32 = 0x7FFFFF;
pub const VRAM_END_U8: &[u8; 8] = b"0x7FFFFF";

/// Address keeping track of current overworld tile, remains at previous value when entering non-ow tile
pub const ADDRESS_OW_TILE_INDEX_U8: &[u8; 8] = b"0x7E008A";
pub const ADDRESS_OW_TILE_INDEX: u32 = 0x7E008A;
/// Address keeping track of current overworld tile, but will shift to 0 when entering non-ow tile
pub const ADDRESS_OW_SLOT_INDEX_U8: &[u8; 8] = b"0x7E040A";
pub const ADDRESS_OW_SLOT_INDEX: u32 = 0x7E008A;
pub const ADDRESS_ENTRANCE_ID_U8: &[u8; 8] = b"0x7E010E";
/// Address that's `1` if Link is inside, `0` if outside;
pub const ADDRESS_IS_INSIDE_U8: &[u8; 8] = b"0x7E001B";

pub fn deserialize_message(buf: &[u8]) -> anyhow::Result<Response> {
    let data =
        &buf[..buf
            .iter()
            .position(|n| n == &MESSAGE_TERMINATOR)
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Tcp message did not have terminator sign",
            ))?];
    let deserialized = serde_json::from_slice(&data)?;
    Ok(deserialized)
}

#[derive(Serialize, Debug)]
pub struct Transition {
    #[serde(with = "ts_milliseconds")]
    timestamp: DateTime<Utc>,
    indoors: bool,
    to: u16,
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn two_bytes_test() {
        let message = Response { data: vec![160, 9] };
        assert_eq!(message.two_bytes(0), 2464);
    }

    macro_rules! test_deserialize_message {
        ($($name:ident: $values:expr,)*) => {
            $(
                #[test]
                fn $name() {
                    let (byte_array, expected) = $values;
                    assert_eq!(deserialize_message(&byte_array).unwrap(), expected)
                }
            )*
        };
    }

    test_deserialize_message! {
        single_val_array: ([123, 34, 100, 97, 116, 97, 34, 58, 32, 91, 49, 49, 55, 93, 125, 10], Response {data: vec![117]}),
        trailing_zeros_are_ignored: ([123, 34, 100, 97, 116, 97, 34, 58, 32, 91, 49, 49, 55, 93, 125, 10, 0, 0, 0, 0, 0, 0], Response {data: vec![117]}),
        multi_val_array: (b"{\"data\": [5, 255]}\n".as_ref(), Response {data: vec![5, 255]}),
    }

    #[test]
    fn missing_end_sign_throws_error() {
        let data: [u8; 15] = [
            123, 34, 100, 97, 116, 97, 34, 58, 32, 91, 49, 49, 55, 93, 125,
        ];
        assert_eq!(
            deserialize_message(&data).unwrap_err().to_string(),
            "Tcp message did not have terminator sign",
        );
    }
}
