use crate::bus::Device;
use heapless::String;

/* TODO
pub enum GbcMode {
    Gb,
    GbGbc,
    OnlyGbc
}
*/

#[derive(Debug)]
pub struct CartridgeHeader {
    pub title: String<16>,
    pub manufacturer_code: String<16>,
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

pub trait Cartridge: Device {
    fn get_header(&self) -> CartridgeHeader {
        let title_iter = (0x134..=0x143)
            .into_iter()
            .map(|addr| self.read(addr) as char);
        let manufacturer_iter = (0x13F..=0x143)
            .into_iter()
            .map(|addr| self.read(addr) as char);

        /* TODO
        let gbc_flag = match self.read(0x143) {
            0x80 => GbcMode::GbGbc,
            0xC0 => GbcMode::OnlyGbc,
            _ => GbcMode::Gb,
        };
        */

        CartridgeHeader {
            title: String::from_iter(title_iter),
            manufacturer_code: String::from_iter(manufacturer_iter),
            //gbc_flag,
            licensee_code: String::new(),
            is_sgb: self.read(0x146) != 0x03,
            cart_type: self.read(0x147),
        }
    }
}

const ROM_LEN: usize = 0x4000;
//const TETRIS: &[u8; 2 * ROM_LEN] = include_bytes!("../roms/tetris.gb");
const ACID: &[u8; 2 * ROM_LEN] = include_bytes!("../testroms/dmg-acid2.gb");

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

impl Cartridge for Rom {}

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
