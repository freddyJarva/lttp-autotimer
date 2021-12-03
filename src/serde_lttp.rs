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
#[allow(unused)]
pub fn hex_16bit_deserialize<'de, D>(d: D) -> Result<u16, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(d)?;
    u16::from_str_radix(&s[2..], 16).map_err(D::Error::custom)
}

pub fn hex_16bit_option_deserialize<'de, D>(d: D) -> Result<Option<u16>, D::Error>
where
    D: Deserializer<'de>,
{
    let so: Option<&str> = Deserialize::deserialize(d)?;
    match so {
        Some(s) => {
            let res = u16::from_str_radix(&s[2..], 16).map_err(D::Error::custom);
            Ok(Some(res?))
        }
        None => Ok(None),
    }
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

pub fn hex_usize_deserialize<'de, D>(d: D) -> Result<usize, D::Error>
where
    D: Deserializer<'de>,
{
    let s: &str = Deserialize::deserialize(d)?;
    usize::from_str_radix(&s[2..], 16).map_err(D::Error::custom)
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
    use serde_json::json;

    use crate::{
        assert_attrs,
        condition::{Conditions, Coordinate},
        transition::Tile,
    };

    #[test]
    fn parse_tile() {
        let json_val = json!({
            "name": "Hyrule Castle - Key Guard 1",
            "indoors": true,
            "address_value": [
                "0x3",
                "0x4",
                "0x5"
            ],
            "conditions": [
                {
                    "type": "Coordinates",
                    "coordinates": [
                        {
                            "x": "1273",
                            "y": "3664-3665",
                            "type": "Range"
                        },
                        {
                            "x": "1272",
                            "y": "3800",
                            "type": "Pair"
                        }
                    ]
                }
            ]
        });
        let parsed: Tile = serde_json::from_str(&json_val.to_string()).unwrap();
        assert_attrs!(
            parsed: name == "Hyrule Castle - Key Guard 1",
            indoors == true,
            address_value == vec![0x3, 0x4, 0x5],
        );
    }

    #[test]
    fn parse_coordinates() {
        let json_val = json!({
            "x": "1272",
            "y": "3800",
            "type": "Pair"
        });
        let coordinate: Coordinate = serde_json::from_str(&json_val.to_string()).unwrap();
        assert_eq!(coordinate, Coordinate::Pair { x: 1272, y: 3800 });
    }

    #[test]
    fn parse_conditions() {
        let json_val = json!(
        [
            {
                "type": "Coordinates",
                "coordinates": [
                    {
                        "type": "Pair",
                        "x": "1272",
                        "y": "568"
                    },
                    {
                        "type": "Pair",
                        "x": "1272",
                        "y": "960"
                    },
                    {
                        "type": "Pair",
                        "x": "1272",
                        "y": "668"
                    }
                ]
            }
        ]
        );

        let _: Vec<Conditions> = serde_json::from_str(&json_val.to_string()).unwrap();
    }
}
