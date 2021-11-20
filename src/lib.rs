use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Message {
    data: Vec<usize>,
}

impl Message {
    pub fn two_bytes(&self) -> usize {
        self.data[0] + (255 * self.data[1])
    }
}

const MESSAGE_END_SIGN: u8 = 10;

pub fn deserialize_message(buf: &[u8]) -> anyhow::Result<Message> {
    let data = &buf[..buf.iter().position(|n| n == &MESSAGE_END_SIGN).unwrap()];

    let deserialized = serde_json::from_slice(&data)?;
    Ok(deserialized)
}

#[cfg(test)]
mod tests {
    #[test]
    fn two_bytes_test() {
        // Given
        let byte_array: Vec<u8> = vec![160,9];
        256 128 64 32 16 8 4 2 1
    }
}
