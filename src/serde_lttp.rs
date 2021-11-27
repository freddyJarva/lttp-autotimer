use serde::{de::Error, Deserialize, Deserializer, Serializer};

#[allow(unused)]
pub fn hex_serialize<S>(x: &u16, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_str(format!("{:X}", x).as_ref())
}

#[allow(unused)]
pub fn hex_serialize_option<S>(x: &Option<u16>, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match x {
        Some(x) => s.serialize_str(format!("{:X}", x).as_ref()),
        None => s.serialize_none(),
    }
}

/// Terrible deserializer of hex values. But hey, it works. I guess.
pub fn hex_deserialize<'de, D>(d: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(d)?;
    u32::from_str_radix(&s[2..], 16).map_err(D::Error::custom)
}

/// Terrible deserializer of hex values. But hey, it works. I guess.
pub fn hex_16bit_deserialize<'de, D>(d: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(d)?;
    u16::from_str_radix(&s[2..], 16).map_err(D::Error::custom)
}

/// Terrible deserializer of 1 byte hex values. But hey, it works. I guess.
pub fn hex_byte_deserialize<'de, D>(d: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(d)?;
    u8::from_str_radix(&s[2..], 16).map_err(D::Error::custom)
}

pub fn coordinate_deserialize<'de, D>(d: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(d)?;
    u16::from_str_radix(&s, 10).map_err(D::Error::custom)
}
