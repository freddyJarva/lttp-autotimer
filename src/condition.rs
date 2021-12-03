use crate::serde_lttp::coordinate_deserialize;
use crate::serde_lttp::coordinate_range_deserialize;
use crate::serde_lttp::hex_16bit_deserialize;
use crate::snes::NamedAddresses;
use crate::transition::Tile;
use crate::SnesRam;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Clone, Hash, Eq)]
pub struct ConditionTransition {
    pub name: String,
    #[serde(deserialize_with = "hex_16bit_deserialize")]
    pub address_value: u16,
    pub indoors: bool,
}

/// Extra conditions to evaluate to deem that a specific transition has been triggered.
///
/// As an example, some tiles use the same address AND address value.
/// In these cases we can evaluate which specific tile Link enters by checking that the previous address value
/// equals one defined in `previous_tiles`
#[derive(Debug, Default, Deserialize, PartialEq, Clone, Hash, Eq)]
pub struct Conditions {
    pub previous_tile: Option<ConditionTransition>,
    pub coordinates: Option<Vec<Coordinate>>,
}

#[derive(Debug, Deserialize, PartialEq, Clone, Hash, Eq)]
#[serde(tag = "type")]
pub enum Coordinate {
    Pair {
        #[serde(deserialize_with = "coordinate_deserialize")]
        x: u16,
        #[serde(deserialize_with = "coordinate_deserialize")]
        y: u16,
    },
    Range {
        #[serde(deserialize_with = "coordinate_range_deserialize")]
        x: (u16, u16),
        #[serde(deserialize_with = "coordinate_range_deserialize")]
        y: (u16, u16),
    },
}

impl Coordinate {
    /// Matches `self` against another `Coordinate` enum returns `true` if criteria is fulfilled
    ///
    /// matching `Coordinate::Pair` against `Coordinate::Pair` is a straightforward equality check.
    ///
    /// when `self` is of type `Pair` and `other` of type `Range`, a match will be found
    /// if the coordinates in `self` is bounded by the coordinate ranges in `other`
    pub fn matches(&self, other: &Self) -> bool {
        match self {
            Coordinate::Pair { x, y } => match other {
                Coordinate::Pair { x: _, y: _ } => self == other,
                Coordinate::Range {
                    x: x_range,
                    y: y_range,
                } => x >= &x_range.0 && x <= &x_range.1 && y >= &y_range.0 && y <= &y_range.1,
            },
            Coordinate::Range { x: _, y: _ } => todo!(),
        }
    }
}

impl From<&SnesRam> for Coordinate {
    fn from(ram: &SnesRam) -> Self {
        Self::Pair {
            x: ram.transition_x(),
            y: ram.transition_y(),
        }
    }
}

pub fn coordinate_condition_met(conditions: &Conditions, current: &SnesRam) -> bool {
    if let Some(condition_coordinate) = &conditions.coordinates {
        condition_coordinate
            .iter()
            .any(|c| Coordinate::from(current).matches(c))
    } else {
        false
    }
}

pub fn previous_tile_condition_met(
    conditions: &Conditions,
    previous_tile: &Tile,
    tile: &Tile,
) -> bool {
    if let Some(previous_tile_condition) = &conditions.previous_tile {
        previous_tile_condition.name == previous_tile.name || tile.name == previous_tile.name
    } else {
        false
    }
}
