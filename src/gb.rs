use crate::bus::Bus;
use crate::cart::{get_cart_header, CartridgeData};
use crate::cpu::Cpu;
use crate::ppu::SCREEN_HEIGHT;
use heapless::Vec;

const CYCLES_PER_FRAME: i32 = 17556;

pub struct GbRs<T: CartridgeData> {
    pub cpu: Cpu<T>,
}

impl<T: CartridgeData> GbRs<T> {
    pub fn new(cart: T) -> Self {
        Self {
            cpu: Cpu::new(Bus::new(cart)),
        }
    }

    pub fn run_one(&mut self) -> usize {
        self.cpu.run_one()
    }

    pub fn run_line(&mut self) {
        // Cycles per line
        let mut cyc_remaining: i32 = CYCLES_PER_FRAME / SCREEN_HEIGHT as i32;
        while cyc_remaining > 0 {
            cyc_remaining -= self.run_one() as i32;
        }
    }

    pub fn run_frame(&mut self) {
        let mut cyc_remaining: i32 = CYCLES_PER_FRAME;
        while cyc_remaining > 0 {
            cyc_remaining -= self.run_one() as i32;
        }
    }
}

const ROM_SIZE: usize = 0x8000;

// A small in memory cartridge implementation
// suitable pretty much only for MBC type 0
pub struct SmallInMemoryCartridge {
    // Not sure arrays because
    // they don't implement DeRef???
    pub rom: Vec<u8, ROM_SIZE>,
    pub ram: Vec<u8, 0>,
}

impl SmallInMemoryCartridge {
    pub fn from_slice(data: &[u8]) -> Self {
        let header = get_cart_header(data);

        if header.rom_size as usize > ROM_SIZE {
            panic!("The size of this ROM is too large for this cartridge implementation!");
        }

        if header.ram_size > 0 {
            panic!("This cartridge does not support RAM!");
        }

        let mut ram = Vec::new();
        ram.resize(ram.capacity(), 0).expect("Unable to resize RAM");

        Self {
            rom: Vec::from_slice(data).expect("Building rom failed?"),
            ram,
        }
    }
}

impl CartridgeData for SmallInMemoryCartridge {
    type Rom = Vec<u8, ROM_SIZE>;
    type Ram = Vec<u8, 0>;

    fn rom(&self) -> &Self::Rom {
        &self.rom
    }

    fn rom_mut(&mut self) -> &mut Self::Rom {
        &mut self.rom
    }

    fn ram(&self) -> &Self::Ram {
        &self.ram
    }

    fn ram_mut(&mut self) -> &mut Self::Ram {
        &mut self.ram
    }
}
