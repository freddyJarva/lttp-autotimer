use byteorder::{ByteOrder, LittleEndian};
use serde::Deserialize;

#[derive(Deserialize, Debug, PartialEq)]
pub struct Message {
    data: Vec<u8>,
}

impl Message {
    pub fn two_bytes(&self, offset: usize) -> u16 {
        LittleEndian::read_u16(&self.data[offset..offset + 2])
    }
}

/// `u8` Value of `\n` in byte string
pub const MESSAGE_TERMINATOR: u8 = 10;
/// Address keeping track of current overworld tile, remains at previous value when entering non-ow tile
pub const ADDRESS_OW_TILE_INDEX: u32 = 0x7E008A;
/// Address keeping track of current overworld tile, but will shift to 0 when entering non-ow tile
pub const ADDRESS_OW_SLOT_INDEX: u32 = 0x7E040A;

pub fn deserialize_message(buf: &[u8]) -> anyhow::Result<Message> {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn two_bytes_test() {
        let message = Message { data: vec![160, 9] };
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
        single_val_array: ([123, 34, 100, 97, 116, 97, 34, 58, 32, 91, 49, 49, 55, 93, 125, 10], Message {data: vec![117]}),
        trailing_zeros_are_ignored: ([123, 34, 100, 97, 116, 97, 34, 58, 32, 91, 49, 49, 55, 93, 125, 10, 0, 0, 0, 0, 0, 0], Message {data: vec![117]}),
        multi_val_array: (b"{\"data\": [5, 255]}\n".as_ref(), Message {data: vec![5, 255]}),
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
