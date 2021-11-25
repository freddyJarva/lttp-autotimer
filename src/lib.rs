pub mod check;
pub mod output;
pub mod qusb;
pub mod snes;
pub mod transition;

/// Snes memory address
pub const VRAM_START: u32 = 0xf50000;
pub const SAVEDATA_START: u32 = VRAM_START + 0xF000;
/// I'm too lazy to manually translate dunka's values, so I'll just use this instead to read from the correct memory address
pub const DUNKA_VRAM_READ_OFFSET: u32 = SAVEDATA_START + 0x280;

/// Address keeping track of current overworld tile, remains at previous value when entering non-ow tile
pub const ADDRESS_OW_SLOT_INDEX: u32 = 0x7E040A;
/// Address keeping track of latest entrance transition, i.e. walking in or out of house/dungeon/etc
pub const ADDRESS_ENTRANCE_ID: u32 = 0x7E010E;
/// Address that's `1` if Link is inside, `0` if outside;
pub const ADDRESS_IS_INSIDE: u32 = 0x7E001B;
