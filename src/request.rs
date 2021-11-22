use cached::proc_macro::cached;
use std::io::{Read, Write};

use byteorder::{ByteOrder, LittleEndian};
use serde::Deserialize;

/// `u8` Value of `\n` in byte string
pub const MESSAGE_TERMINATOR: u8 = b'\n';
pub const FIELD_DELIMITER: u8 = b'|';
pub const ADDRESS_DELIMITER: u8 = b',';

#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub enum RequestType {
    Read,
    Write,
}

#[derive(Debug, PartialEq, Hash, Eq, Clone)]
pub struct RequestBody {
    pub request_type: RequestType,
    pub addresses: Vec<Vec<u8>>,
    pub address_length: u8,
    pub device_type: String,
}

impl RequestBody {
    pub fn serialize(&mut self) -> Vec<u8> {
        serialize_body(self.clone())
    }
}

#[cached]
fn serialize_body(body: RequestBody) -> Vec<u8> {
    let mut serialized: Vec<u8> = match body.request_type {
        RequestType::Read => b"READ".into_iter().map(|n| *n).collect(),
        RequestType::Write => b"WRITE".into_iter().map(|n| *n).collect(),
    };
    serialized.push(FIELD_DELIMITER);
    for address in body.addresses {
        serialized.extend_from_slice(&address);
        serialized.push(ADDRESS_DELIMITER);
    }
    serialized.push(FIELD_DELIMITER);
    serialized.push(body.address_length as u8);
    serialized.push(FIELD_DELIMITER);
    serialized.extend_from_slice(b"System Bus");
    serialized.push(MESSAGE_TERMINATOR);

    serialized
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct Response {
    pub data: Vec<u8>,
}

impl Response {
    pub fn two_bytes(&self, offset: usize) -> u16 {
        LittleEndian::read_u16(&self.data[offset..offset + 2])
    }
}

pub fn two_byte_addresses<S: AsRef<str>, T: Write + Read>(
    stream: &mut T,
    buf: &mut [u8],
    memory_addresses: &[S],
) -> anyhow::Result<usize> {
    stream.write(b"READ|0x7E040A,0x7E008A|2|System Bus\n")?;
    let res = stream.read(buf)?;
    Ok(res)
}

#[cfg(test)]
mod tests {
    use std::char::from_digit;

    use super::*;

    #[test]
    fn test_serialize() {
        let mut body = RequestBody {
            request_type: RequestType::Read,
            addresses: vec![b"0x7E040A".to_vec(), b"0x7E008A".to_vec()],
            address_length: b'2',
            device_type: "System Bus".to_string(),
        };
        let expected = b"READ|0x7E040A,0x7E008A,|2|System Bus\n";
        assert_eq!(body.serialize(), expected)
    }
}
