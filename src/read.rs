use std::io::{Read, Write};

use crate::{
    deserialize_message,
    request::{self, RequestBody, RequestType, Response, DEVICE_TYPE_SYSTEM_BUS},
    VRAM_END, VRAM_START, VRAM_START_U8,
};

pub fn overworld_location<S: AsRef<[u8]>, T: Write + Read>(
    stream: &mut T,
    buf: &mut [u8],
    memory_addresses: Vec<S>,
) -> anyhow::Result<Response> {
    request::two_byte_addresses(stream, buf, memory_addresses)?;
    let deserialized = deserialize_message(&buf)?;
    Ok(deserialized)
}

pub fn vram<S: AsRef<[u8]>, T: Write + Read>(
    stream: &mut T,
    buf: &mut [u8],
) -> anyhow::Result<Response> {
    let mut body = RequestBody {
        request_type: RequestType::Read,
        addresses: vec![VRAM_START_U8.to_vec()],
        address_length: VRAM_END - VRAM_START,
        device_type: DEVICE_TYPE_SYSTEM_BUS.to_string(),
    };
    stream.write(&body.serialize())?;
    stream.read(buf)?;
    let res = deserialize_message(&buf)?;
    Ok(res)
}
