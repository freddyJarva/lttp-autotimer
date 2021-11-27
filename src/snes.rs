use crate::{
    COORDINATE_CHUNK_SIZE, COORDINATE_OFFSET, DUNKA_CHUNK_SIZE, DUNKA_OFFSET, TILE_INFO_CHUNK_SIZE,
};

use byteorder::{BigEndian, ByteOrder};

const OVERWORLD_TILE_ADDRESS: usize = 0x40a;
const ENTRANCE_ID_ADDRESS: usize = 0x10E;
const INDOORS_ADDRESS: usize = 0x1b;

pub trait NamedAddresses {
    fn overworld_tile(&self) -> u8;
    fn entrance_id(&self) -> u8;
    fn indoors(&self) -> u8;
    fn x(&self) -> u16;
    fn y(&self) -> u16;
}

pub trait SetNamedAddresses {
    fn set_overworld_tile(&mut self, byte: u8);
    fn set_entrance_id(&mut self, byte: u8);
    fn set_indoors(&mut self, byte: u8);
    fn set_x(&mut self, word: u16);
    fn set_y(&mut self, word: u16);
}

impl NamedAddresses for SnesRam {
    fn overworld_tile(&self) -> u8 {
        self.tile_info_chunk[OVERWORLD_TILE_ADDRESS]
    }

    fn entrance_id(&self) -> u8 {
        self.tile_info_chunk[ENTRANCE_ID_ADDRESS]
    }

    fn indoors(&self) -> u8 {
        self.tile_info_chunk[INDOORS_ADDRESS]
    }

    fn x(&self) -> u16 {
        BigEndian::read_u16(&self.coordinate_chunk[2..])
    }

    fn y(&self) -> u16 {
        BigEndian::read_u16(&self.coordinate_chunk[..2])
    }
}

impl SetNamedAddresses for SnesRam {
    fn set_overworld_tile(&mut self, byte: u8) {
        self.tile_info_chunk[OVERWORLD_TILE_ADDRESS] = byte
    }

    fn set_entrance_id(&mut self, byte: u8) {
        self.tile_info_chunk[ENTRANCE_ID_ADDRESS] = byte
    }

    fn set_indoors(&mut self, byte: u8) {
        self.tile_info_chunk[INDOORS_ADDRESS] = byte
    }

    fn set_x(&mut self, word: u16) {
        todo!()
    }

    fn set_y(&mut self, word: u16) {
        todo!()
    }
}

impl NamedAddresses for &SnesRam {
    fn overworld_tile(&self) -> u8 {
        self.tile_info_chunk[OVERWORLD_TILE_ADDRESS]
    }

    fn entrance_id(&self) -> u8 {
        self.tile_info_chunk[ENTRANCE_ID_ADDRESS]
    }

    fn indoors(&self) -> u8 {
        self.tile_info_chunk[INDOORS_ADDRESS]
    }

    fn x(&self) -> u16 {
        todo!()
    }

    fn y(&self) -> u16 {
        todo!()
    }
}

impl NamedAddresses for Vec<u8> {
    fn overworld_tile(&self) -> u8 {
        self[OVERWORLD_TILE_ADDRESS]
    }

    fn entrance_id(&self) -> u8 {
        self[ENTRANCE_ID_ADDRESS]
    }

    fn indoors(&self) -> u8 {
        self[INDOORS_ADDRESS]
    }

    fn x(&self) -> u16 {
        todo!()
    }

    fn y(&self) -> u16 {
        todo!()
    }
}

impl NamedAddresses for [u8] {
    fn overworld_tile(&self) -> u8 {
        self[OVERWORLD_TILE_ADDRESS]
    }

    fn entrance_id(&self) -> u8 {
        self[ENTRANCE_ID_ADDRESS]
    }

    fn indoors(&self) -> u8 {
        self[INDOORS_ADDRESS]
    }

    fn x(&self) -> u16 {
        todo!()
    }

    fn y(&self) -> u16 {
        todo!()
    }
}

/// Handles values read from qusb while maintaining correct address locations relative to VRAM_START
///
/// This allows us to load only the parts we want from qusb while not having to handle tons of different offset values in a myriad of places
pub struct SnesRam {
    pub tile_info_chunk: Vec<u8>,
    pub dunka_chunka: Vec<u8>,
    pub coordinate_chunk: Vec<u8>,
}

impl SnesRam {
    /// addresses are relative to `VRAM_START` (`0xf50000`)
    pub fn get_byte(&self, address: usize) -> u8 {
        if address < TILE_INFO_CHUNK_SIZE {
            self.tile_info_chunk[address]
        } else if address >= DUNKA_OFFSET && address < DUNKA_OFFSET + DUNKA_CHUNK_SIZE {
            self.dunka_chunka[address - DUNKA_OFFSET]
        } else if address >= COORDINATE_OFFSET
            && address < COORDINATE_OFFSET + COORDINATE_CHUNK_SIZE
        {
            self.coordinate_chunk[address - COORDINATE_OFFSET]
        } else {
            panic!("Tried to read value from address not fetched from qusb!")
        }
    }

    pub fn new() -> Self {
        Self {
            tile_info_chunk: vec![],
            dunka_chunka: vec![],
            coordinate_chunk: vec![],
        }
    }
}

/// Right now only used for testing purposes to create fake snes reads
pub struct SnesRamInitializer {
    pub overworld_tile: Option<u8>,
    pub entrance_id: Option<u8>,
    pub indoors: Option<u8>,
    pub x: Option<u16>,
    pub y: Option<u16>,
}

impl SnesRamInitializer {
    fn build() -> Vec<u8> {
        vec![0; 0x40b]
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn setting_xy_sets_two_bytes_each() {}
}
