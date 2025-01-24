use crate::cart::CartridgeData;
use crate::cart::MBC1;

pub struct MBC5 {
    mbc1: MBC1,
}

impl MBC5 {
    pub fn new() -> Self {
        Self { mbc1: MBC1::new() }
    }

    pub fn write<Cart: CartridgeData>(&mut self, cart: &mut Cart, addr: u16, val: u8) {
        self.mbc1.write(cart, addr, val);
        //TODO: RTC and stuff
    }

    pub fn read<Cart: CartridgeData>(&self, cart: &Cart, addr: u16) -> u8 {
        self.mbc1.read(cart, addr)
        //TODO: RTC and stuff
    }
}
