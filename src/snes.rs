pub trait NamedAddresses {
    fn overworld_tile(&self) -> u8;
    fn entrance_id(&self) -> u8;
    fn indoors(&self) -> u8;
}

impl NamedAddresses for Vec<u8> {
    fn overworld_tile(&self) -> u8 {
        self[0x40A]
    }

    fn entrance_id(&self) -> u8 {
        self[0x10E]
    }

    fn indoors(&self) -> u8 {
        self[0x1B]
    }
}
