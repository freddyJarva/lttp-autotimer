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

pub fn hex_16bit_array_deserialize<'de, D>(d: D) -> Result<Vec<u16>, D::Error>
where
    D: Deserializer<'de>,
{
    let strings: Vec<&str> = Deserialize::deserialize(d)?;
    Ok(strings
        .into_iter()
        .map(|s| u16::from_str_radix(&s[2..], 16).unwrap())
        .collect())
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

pub fn coordinate_range_deserialize<'de, D>(d: D) -> Result<(u16, u16), D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(d)?;
    let vals: Vec<u16> = s
        .split('-')
        .into_iter()
        .filter_map(|c| u16::from_str_radix(c, 10).map_err(D::Error::custom).ok())
        .collect();
    if vals.len() == 1 {
        // If single value defined in json, handle it as a 1 value range, inclusive min & max
        Ok((vals[0], vals[0]))
    } else {
        Ok((vals[0], vals[1]))
    }
}

#[cfg(test)]
mod tests {
    use crate::{assert_attrs, transition::Tile};

    #[test]
    fn testname() {
        let parsed: Tile = serde_json::from_str(
            "
        {
            \"name\": \"Sewers - Bonk Walls\",
            \"indoors\": true,
            \"address_value\": [
                \"0x3\",
                \"0x4\",
                \"0x5\",
                \"0x81\"
            ],
            \"conditions\": {
                \"coordinates\": [
                    {
                        \"type\": \"Pair\",
                        \"x\": \"888\",
                        \"y\": \"984\"
                    },
                    {
                        \"type\": \"Pair\",
                        \"x\": \"808\",
                        \"y\": \"888\"
                    },
                    {
                        \"type\": \"Pair\",
                        \"x\": \"808\",
                        \"y\": \"632\"
                    },
                    {
                        \"type\": \"Range\",
                        \"x\": \"889\",
                        \"y\": \"567-569\"
                    },
                    {
                        \"type\": \"Range\",
                        \"x\": \"889\",
                        \"y\": \"567-569\"
                    }
                ]
            }
        }
        ",
        )
        .unwrap();
        assert_attrs!(
            parsed: name == "Sewers - Bonk Walls",
            indoors == true,
            address_value == vec![0x3, 0x4, 0x5, 0x81],
        );
    }
}
