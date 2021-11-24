pub mod qusb;
pub mod snes;
pub mod transition;

/// Snes memory address
pub const VRAM_START: u32 = 0x7E0000;
pub const VRAM_START_U8: &[u8; 8] = b"0x7E0000";
/// Snes memory address
pub const VRAM_END: u32 = 0x7FFFFF;
pub const VRAM_END_U8: &[u8; 8] = b"0x7FFFFF";

/// Address keeping track of current overworld tile, remains at previous value when entering non-ow tile
pub const ADDRESS_OW_TILE_INDEX_U8: &[u8; 8] = b"0x7E008A";
pub const ADDRESS_OW_TILE_INDEX: u32 = 0x7E008A;
/// Address keeping track of current overworld tile, but will shift to 0 when entering non-ow tile
pub const ADDRESS_OW_SLOT_INDEX_U8: &[u8; 8] = b"0x7E040A";
pub const ADDRESS_OW_SLOT_INDEX: u32 = 0x7E040A;
pub const ADDRESS_ENTRANCE_ID_U8: &[u8; 8] = b"0x7E010E";
pub const ADDRESS_ENTRANCE_ID: u32 = 0x7E010E;
/// Address that's `1` if Link is inside, `0` if outside;
pub const ADDRESS_IS_INSIDE_U8: &[u8; 8] = b"0x7E001B";
pub const ADDRESS_IS_INSIDE: u32 = 0x7E001B;
