use crate::bus::Device;
use heapless::String;
use heapless::Vec;

/* TODO
pub enum GbcMode {
    Gb,
    GbGbc,
    OnlyGbc
}
*/

#[derive(Debug)]
pub struct CartridgeHeader {
    pub title: String<20>,
    pub manufacturer_code: String<20>,
    //pub gbc_flag: GbcMode,
    pub licensee_code: String<16>,
    pub is_sgb: bool,
    pub cart_type: u8,
    /* TODO
    pub rom_size: u32,
    pub ram_size: u32,
    pub dest_code: bool
    */
}

pub trait RamController: Device {}

pub struct NoRam {}

impl Device for NoRam {
    fn write(&mut self, _addr: u16, _val: u8) {
        //No Op
    }

    fn read(&self, _addr: u16) -> u8 {
        0xFF
        //No Op
    }
}

impl RamController for NoRam {}

pub struct Cartridge<const MAX_ROM: usize, T: RamController> {
    rom: Vec<u8, MAX_ROM>,
    ram: T,
}

pub type SimpleCart = Cartridge<0x8000, NoRam>; // 32K
pub type FullRom = Cartridge<0x800000, NoRam>; // 8MiB

impl SimpleCart {
    pub fn acid_cart() -> Self {
        const ACID: &[u8; 0x8000] = include_bytes!("../tests/roms/dmg-acid2.gb");
        Self {
            rom: Vec::from_slice(ACID).expect("DMG-Acid2 failure to load"),
            ram: NoRam {},
        }
    }

    pub fn from_slice(data: &[u8]) -> Self {
        let cart = Self {
            rom: Vec::from_slice(data).expect("Failed to load cart from slice"),
            ram: NoRam {},
        };

        cart
    }
}

impl<const MAX_ROM: usize, T: RamController> Device for Cartridge<MAX_ROM, T> {
    fn write(&mut self, _addr: u16, _val: u8) {
        //No Op
    }

    fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        self.rom[addr]
    }
}

const ROM_LEN: usize = 0x4000;
//const TETRIS: &[u8; 2 * ROM_LEN] = include_bytes!("../roms/tetris.gb");
const ACID: &[u8; 2 * ROM_LEN] = include_bytes!("../tests/roms/dmg-acid2.gb");

pub struct Rom {
    rom: [u8; ROM_LEN],
    mapped_rom: [u8; ROM_LEN],
}

impl Rom {
    /*
    pub fn tetris_cart() -> Rom {
        Self {
            rom: TETRIS[..ROM_LEN].try_into().unwrap(),
            mapped_rom: TETRIS[ROM_LEN..].try_into().unwrap(),
        }
    }
    */

    pub fn acid_cart() -> Rom {
        Self {
            rom: ACID[..ROM_LEN].try_into().unwrap(),
            mapped_rom: ACID[ROM_LEN..].try_into().unwrap(),
        }
    }

    pub fn from_slice(data: &[u8]) -> Rom {
        let mut rom = Rom {
            rom: [0; ROM_LEN],
            mapped_rom: [0; ROM_LEN],
        };

        rom.rom.copy_from_slice(&data[..ROM_LEN]);
        rom.mapped_rom.copy_from_slice(&data[ROM_LEN..]);
        rom
    }
}

impl Device for Rom {
    fn write(&mut self, _addr: u16, _val: u8) {
        //No Op
    }

    fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        if addr < ROM_LEN {
            self.rom[addr]
        } else {
            self.mapped_rom[addr - ROM_LEN]
        }
    }
}
