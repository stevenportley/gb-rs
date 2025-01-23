use crate::bus::Device;
use core::ops::DerefMut;
use heapless::String;

mod mbc1;
mod mbc5;

use mbc1::MBC1;
use mbc5::MBC5;

pub trait CartridgeData {
    type Rom: DerefMut<Target = [u8]> + ?Sized;
    type Ram: DerefMut<Target = [u8]> + ?Sized;

    fn rom(&self) -> &Self::Rom;
    fn rom_mut(&mut self) -> &mut Self::Rom;
    fn ram(&self) -> &Self::Ram;
    fn ram_mut(&mut self) -> &mut Self::Ram;

    fn get_header(&self) -> CartridgeHeader {
        get_cart_header(self.rom())
    }
}

enum MemoryBankController {
    MBC0,
    MBC1(MBC1),
    MBC5(MBC5),
}

pub struct Cartridge<T: CartridgeData> {
    data: T,
    mbc: MemoryBankController,
}

impl<T: CartridgeData> Cartridge<T> {
    pub fn new(data: T) -> Self {
        let header = data.get_header();

        let mbc: MemoryBankController = match header.cart_type {
            0 => MemoryBankController::MBC0,
            1 | 2 | 3 => MemoryBankController::MBC1(MBC1::new()),
            0x13 => MemoryBankController::MBC5(MBC5::new()),
            0x19..0x1E => MemoryBankController::MBC5(MBC5::new()),
            _ => {
                unimplemented!("Unimplemented MBC type")
            }
        };

        Self { data, mbc }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match &mut self.mbc {
            MemoryBankController::MBC0 => { /* NOP */ }
            MemoryBankController::MBC1(mbc) => mbc.write(&mut self.data, addr, val),
            MemoryBankController::MBC5(mbc) => mbc.write(&mut self.data, addr, val),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match &self.mbc {
            MemoryBankController::MBC0 => self.data.rom()[addr as usize],
            MemoryBankController::MBC1(mbc) => mbc.read(&self.data, addr),
            MemoryBankController::MBC5(mbc) => mbc.read(&self.data, addr),
        }
    }

    pub fn get_header(&self) -> CartridgeHeader {
        self.data.get_header()
    }
}

#[derive(Debug)]
pub struct CartridgeHeader {
    pub title: String<25>,
    pub manufacturer_code: String<16>,
    //pub gbc_flag: GbcMode,
    pub licensee_code: String<16>,
    pub is_sgb: bool,
    pub cart_type: u8,
    pub rom_size: u32,
    pub ram_size: u32,
    pub num_rom_banks: u16,
    /* TODO
    pub dest_code: bool
    */
}

pub fn get_cart_header(rom: &[u8]) -> CartridgeHeader {
    let title = (0x134..=0x143)
        .into_iter()
        .map(|addr| rom[addr])
        .take_while(|b| *b != 0)
        .collect();
    let title = String::from_utf8(title).expect("The title is invalid UTF-8");

    let manufacturer_code = (0x13F..=0x143)
        .into_iter()
        .map(|addr| rom[addr])
        .take_while(|b| *b != 0)
        .collect();
    let manufacturer_code =
        String::from_utf8(manufacturer_code).expect("The manufacturer is invalid UTF-8");

    let rom_size = 32768 * (1 << rom[0x148]);
    let ram_size = match rom[0x149] {
        0 => 0,
        1 => unreachable!("Invalid amount of RAM"),
        2 => 8192,
        3 => 32768,
        4 => 131072,
        5 => 65536,
        _ => unreachable!("Invalid amount of RAM"),
    };

    // Each ROM bank is 16k
    let num_rom_banks = (rom_size / 16384) as u16;

    CartridgeHeader {
        title,
        manufacturer_code,
        //gbc_flag,
        licensee_code: String::new(),
        is_sgb: rom[0x146] != 0x03,
        cart_type: rom[0x147],
        rom_size,
        ram_size,
        num_rom_banks,
    }
}

/*
 * Preserving this incase we every want to do built-in rom things
 *

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
*/
