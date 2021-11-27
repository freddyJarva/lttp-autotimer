use crate::{SAVEDATA_START, SAVE_DATA_OFFSET, VRAM_START};

const OVERWORLD_TILE_ADDRESS: usize = 0x40a;
const ENTRANCE_ID_ADDRESS: usize = 0x10E;
const INDOORS_ADDRESS: usize = 0x1b;

const X_ADDRESS: usize = 0xC184;
pub trait NamedAddresses {
    fn overworld_tile(&self) -> u8;
    fn entrance_id(&self) -> u8;
    fn indoors(&self) -> u8;
    fn x(&self) -> u16;
    fn y(&self) -> u16;

    fn set_overworld_tile(&mut self, byte: u8);
    fn set_entrance_id(&mut self, byte: u8);
    fn set_indoors(&mut self, byte: u8);
    fn set_x(&mut self, word: u16);
    fn set_y(&mut self, word: u16);
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

    fn set_overworld_tile(&mut self, byte: u8) {
        self[OVERWORLD_TILE_ADDRESS] = byte
    }

    fn set_entrance_id(&mut self, byte: u8) {
        self[ENTRANCE_ID_ADDRESS] = byte
    }

    fn set_indoors(&mut self, byte: u8) {
        self[INDOORS_ADDRESS] = byte
    }

    fn x(&self) -> u16 {
        todo!()
    }

    fn y(&self) -> u16 {
        todo!()
    }

    fn set_x(&mut self, word: u16) {
        todo!()
    }

    fn set_y(&mut self, word: u16) {
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

    fn set_overworld_tile(&mut self, byte: u8) {
        self[OVERWORLD_TILE_ADDRESS] = byte
    }

    fn set_entrance_id(&mut self, byte: u8) {
        self[ENTRANCE_ID_ADDRESS] = byte
    }

    fn set_indoors(&mut self, byte: u8) {
        self[INDOORS_ADDRESS] = byte
    }

    fn x(&self) -> u16 {
        todo!()
    }

    fn y(&self) -> u16 {
        todo!()
    }

    fn set_x(&mut self, word: u16) {
        todo!()
    }

    fn set_y(&mut self, word: u16) {
        todo!()
    }
}

pub fn normalize_dunka(address: usize) -> usize {
    address + SAVE_DATA_OFFSET
}

pub struct SnesRam {
    dunka_chunka: [u8; 0x280 + 0x280],
    tile_info_chunk: [u8; 0x40b],
    coordinate_chunk: [u8; 0x04],
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
