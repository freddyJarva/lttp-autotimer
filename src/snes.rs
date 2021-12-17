use crate::{
    sni::api::ReadMemoryResponse, COORDINATE_CHUNK_SIZE, COORDINATE_OFFSET, DUNKA_CHUNK_SIZE,
    DUNKA_OFFSET, GAME_STATS_OFFSET, GAME_STATS_SIZE, TILE_INFO_CHUNK_SIZE,
};

use byteorder::{ByteOrder, LittleEndian};

const OVERWORLD_TILE_ADDRESS: usize = 0x40a;
const ENTRANCE_ID_ADDRESS: usize = 0x10E;
const INDOORS_ADDRESS: usize = 0x1b;
const GAME_MODE_ADDRESS: usize = 0x95;
const GAME_STATE_ADDRESS: usize = 0x10;

pub trait NamedAddresses {
    fn overworld_tile(&self) -> u8;
    fn entrance_id(&self) -> u8;
    fn indoors(&self) -> u8;
    fn x(&self) -> u16;
    fn y(&self) -> u16;
    fn transition_x(&self) -> u16;
    fn transition_y(&self) -> u16;
    /// 15 (in decimal base) on start screen, 7 when the game is started (Link is spawned into the world), 3 after flying
    fn game_mode(&self) -> u8;
    fn game_state(&self) -> u8;
}

pub trait SetNamedAddresses {
    fn set_overworld_tile(&mut self, byte: u8);
    fn set_entrance_id(&mut self, byte: u8);
    fn set_indoors(&mut self, byte: u8);
    fn set_x(&mut self, word: u16);
    fn set_y(&mut self, word: u16);
    fn set_transition_x(&mut self, word: u16);
    fn set_transition_y(&mut self, word: u16);
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

    fn transition_x(&self) -> u16 {
        LittleEndian::read_u16(&self.coordinate_chunk[2..])
    }

    fn transition_y(&self) -> u16 {
        LittleEndian::read_u16(&self.coordinate_chunk[..2])
    }

    fn game_mode(&self) -> u8 {
        self.tile_info_chunk[GAME_MODE_ADDRESS]
    }

    fn x(&self) -> u16 {
        LittleEndian::read_u16(&self.tile_info_chunk[0x22..0x24])
    }

    fn y(&self) -> u16 {
        LittleEndian::read_u16(&self.tile_info_chunk[0x20..0x22])
    }

    fn game_state(&self) -> u8 {
        self.get_byte(GAME_STATE_ADDRESS)
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

    fn set_transition_x(&mut self, word: u16) {
        LittleEndian::write_u16(&mut self.coordinate_chunk[2..], word)
    }

    fn set_transition_y(&mut self, word: u16) {
        LittleEndian::write_u16(&mut self.coordinate_chunk[..2], word)
    }

    fn set_x(&mut self, word: u16) {
        LittleEndian::write_u16(&mut self.tile_info_chunk[0x22..0x24], word)
    }

    fn set_y(&mut self, word: u16) {
        LittleEndian::write_u16(&mut self.tile_info_chunk[0x20..0x22], word)
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

    fn transition_x(&self) -> u16 {
        LittleEndian::read_u16(&self.coordinate_chunk[2..])
    }

    fn transition_y(&self) -> u16 {
        LittleEndian::read_u16(&self.coordinate_chunk[..2])
    }

    fn game_mode(&self) -> u8 {
        self.tile_info_chunk[GAME_MODE_ADDRESS]
    }

    fn x(&self) -> u16 {
        todo!()
    }

    fn y(&self) -> u16 {
        todo!()
    }

    fn game_state(&self) -> u8 {
        self.get_byte(GAME_STATE_ADDRESS)
    }
}

/// This assumed a vec with the correct order
impl From<&Vec<ReadMemoryResponse>> for SnesRam {
    fn from(responses: &Vec<ReadMemoryResponse>) -> Self {
        let mut snes_ram = SnesRam::new();
        for (idx, response) in responses.iter().enumerate() {
            match idx {
                0 => snes_ram.tile_info_chunk = response.data.clone(),
                1 => snes_ram.dunka_chunka = response.data.clone(),
                2 => snes_ram.game_stats_chunk = response.data.clone(),
                3 => snes_ram.coordinate_chunk = response.data.clone(),
                _ => (),
            }
        }
        snes_ram
    }
}

/// Handles values read from qusb while maintaining correct address locations relative to VRAM_START
///
/// This allows us to load only the parts we want from qusb while not having to handle tons of different offset values in a myriad of places
#[derive(Default, Debug)]
pub struct SnesRam {
    pub tile_info_chunk: Vec<u8>,
    pub dunka_chunka: Vec<u8>,
    pub coordinate_chunk: Vec<u8>,
    pub game_stats_chunk: Vec<u8>,
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
        } else if address >= GAME_STATS_OFFSET && address < GAME_STATS_OFFSET + GAME_STATS_SIZE {
            self.game_stats_chunk[address - GAME_STATS_OFFSET]
        } else {
            panic!(
                "Tried reading address with offset {:X} from ram, but it's not fetched from the game!",
                address
            )
        }
    }

    /// Reads the ram value to see if has started since boot/reset/S&Q
    ///
    /// Reads the value at 0x7e0010, which can be any of these:
    ///
    /// * 00 - Intro
    /// * 01 - File Select
    /// * 02 - Copy File
    /// * 03 - Delete File
    /// * 04 - Name File
    /// * 05 - Load File
    /// * 06 - UnderworldLoad
    /// * 07 - Underworld
    /// * 08 - OverworldLoad
    /// * 09 - Overworld
    /// * 0A - OverworldSpecialLoad
    /// * 0B - OverworldSpecial
    /// * 0C/0D - Unused
    /// * 0E - Interface
    /// * 0F - SpotlightClose
    /// * 10 - SpotlightOpen
    /// * 11 - UnderworldFallingEntrance
    /// * 12 - GameOver
    /// * 13 - BossVictory_Pendant
    /// * 14 - Attract
    /// * 15 - MirrorWarpFromAge
    /// * 16 - BossVictory_Crystal
    /// * 17 - SaveAndQuit
    /// * 18 - GanonEmerges
    /// * 19 - TriforceRoom
    /// * 1A - Credits
    /// * 1B - SpawnSelect
    pub fn game_has_started(&self) -> bool {
        println!("Game has started: {}", self.get_byte(GAME_STATE_ADDRESS));
        match self.get_byte(GAME_STATE_ADDRESS) {
            0x06..=0x0b => true,
            _ => false,
        }
    }

    pub fn new() -> Self {
        Self {
            tile_info_chunk: vec![0; TILE_INFO_CHUNK_SIZE],
            dunka_chunka: vec![0; DUNKA_CHUNK_SIZE],
            coordinate_chunk: vec![0; COORDINATE_CHUNK_SIZE],
            game_stats_chunk: vec![0; GAME_STATS_SIZE],
        }
    }
}

#[cfg(test)]
/// Right now only used for testing purposes to create fake snes reads
#[derive(Default)]
pub struct SnesRamInitializer {
    pub overworld_tile: Option<u8>,
    pub entrance_id: Option<u8>,
    pub indoors: Option<u8>,
    pub transition_x: Option<u16>,
    pub transition_y: Option<u16>,
}

#[cfg(test)]
impl SnesRamInitializer {
    pub fn build(&self) -> SnesRam {
        let mut ram = SnesRam::new();
        ram.set_entrance_id(self.entrance_id.unwrap_or(0));
        ram.set_indoors(self.indoors.unwrap_or(0));
        ram.set_overworld_tile(self.overworld_tile.unwrap_or(0));
        ram.set_transition_x(self.transition_x.unwrap_or(0));
        ram.set_transition_y(self.transition_y.unwrap_or(0));
        ram
    }
}

#[cfg(test)]
mod tests {
    use crate::snes::NamedAddresses;

    use super::{SetNamedAddresses, SnesRam};

    #[test]
    fn test_set_xy() {
        let mut ram = SnesRam::new();
        ram.set_transition_x(12);
        ram.set_transition_y(55000);
        assert_eq!((ram.transition_x(), ram.transition_y()), (12, 55000));
    }
}
