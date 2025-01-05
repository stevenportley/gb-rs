
use core::str::FromStr;

use crate::bus::Device;
use heapless::String;
use heapless::Vec;

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
    /* TODO
    pub dest_code: bool
    */
}

#[derive(Default)]
pub struct SimpleRam {
    pub ram: Rom<8192>,
}

impl Ram for SimpleRam {
    fn write(&mut self, addr: u32, val: u8) {
        self.ram[addr as usize] = val;
    }
    fn read(&self, addr: u32) -> u8 {
        self.ram[addr as usize]
    }

}

pub trait Ram {
    fn write(&mut self, addr: u32, val: u8);
    fn read(&self, addr: u32) -> u8;
}

type Rom<const SIZE: usize> = Vec<u8, SIZE>;

pub enum MBC<const ROM_SIZE: usize> {
    NoMBC { rom: Rom<ROM_SIZE> },
    MBC1 { mbc: MBC1<ROM_SIZE, SimpleRam> },
}

fn get_header(rom: &[u8]) -> CartridgeHeader {
    let title_iter = (0x134..=0x143).into_iter().map(|addr| rom[addr] as char);

    let manufacturer_iter = (0x13F..=0x143).into_iter().map(|addr| rom[addr] as char);

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

    CartridgeHeader {
        title: String::from_iter(title_iter),
        manufacturer_code: String::from_iter(manufacturer_iter),
        //gbc_flag,
        licensee_code: String::new(),
        is_sgb: rom[0x146] != 0x03,
        cart_type: rom[0x147],
        rom_size,
        ram_size,
    }
}

impl<const ROM_SIZE: usize> Device for MBC<ROM_SIZE> {
    fn write(&mut self, addr: u16, val: u8) {
        match self {
            MBC::NoMBC { .. } => { /* NOP */ },
            MBC::MBC1 { mbc } => mbc.write(addr, val),
        }
    }

    fn read(&self, addr: u16) -> u8 {
        match self {
            MBC::NoMBC { rom } => rom[addr as usize],
            MBC::MBC1 { mbc } => mbc.read(addr),
        }
    }
}

impl<const ROM_SIZE: usize> MBC<ROM_SIZE> {

    pub fn new(rom: &[u8]) -> Self {
        let header = get_header(rom);

        match header.cart_type {
            0 => { 
                MBC::NoMBC { rom: Rom::from_slice(rom).expect("Failed to build No MBC") }
            },
            1 | 2 | 3 => {
                let mut a = MBC::MBC1 { mbc: MBC1::new(rom, 
                    SimpleRam{ 
                        ram: Rom::new()
                    }) 
                };

                loop {

                    if let MBC::MBC1 { mbc } = &mut a {
                        if mbc.ram.ram.is_full() {
                            break;
                        }
                        let _ = mbc.ram.ram.push(0);
                    }
                }

                a

            },
            _ => { unimplemented!("MBC type not implemented! Cart type: {:?}", header.cart_type) },
            
        }

    }

    pub fn get_header(&self) -> CartridgeHeader {
        match self {
            MBC::NoMBC { rom } => get_header(rom),
            MBC::MBC1 { mbc } => get_header(mbc.rom.as_slice()),
        }
    }
}

struct MBC1<const ROM_SIZE: usize, R: Ram> {
    ram_en: bool,
    rom_bank_num: u8,
    adv_bank_mode: bool,
    ram_bank_num: u8,
    rom: Rom<ROM_SIZE>,
    ram: R,
}

impl<const ROM_SIZE: usize, R: Ram> MBC1<ROM_SIZE, R> {

    fn new(rom: &[u8], ram: R) -> Self {
        assert!(rom.len() <= ROM_SIZE);
        Self {
            ram_en: false,
            rom_bank_num: 0,
            adv_bank_mode: false,
            ram_bank_num: 1,
            rom: Rom::from_slice(rom).expect("Failed to build ROM"),
            ram,
        }
    }

}

impl<const ROM_SIZE: usize, R: Ram> Device for MBC1<ROM_SIZE, R> {
    fn write(&mut self, addr: u16, val: u8) {
        if addr < 0xA000 {
            //panic!("Writing to RAM: Addr {:?}, Val {:?}", addr, val);
        }
        match addr {
            /* Registers */
            0..=0x1FFF => {
                if (val & 0xF) == 0xA {
                    panic!("Enabling ram");
                    self.ram_en = true
                } else {
                    self.ram_en = false
                }
            }
            0x2000..=0x3FFF => {
                self.rom_bank_num = val & 0x1F;

                // From pandocs:
                // "If the main 5-bit ROM banking register is 0, it reads the bank as if it was set to 1."
                if self.rom_bank_num == 0 {
                    self.rom_bank_num = 1;
                }

            },
            0x4000..=0x5FFF => self.ram_bank_num = val & 0x3,
            0x6000..=0x7FFF => {
                unimplemented!("Not implementing advanced banking mode!");
                self.adv_bank_mode = val & 0x1 == 0x1
            }

            /* Memory banks */
            0xA000..=0xBFFF => {
                if !self.ram_en {
                    //panic!("Writing to disabled RAM");
                    //return;
                }

                let offset = (addr - 0xA000) as u32
                    + if self.adv_bank_mode {
                        (self.ram_bank_num as u32) << 13
                    } else {
                        0
                    };

                self.ram.write(offset, val);
            }
            _ => {
                unreachable!("Invalid MBC1 address! addr: {:?}, val: {:?}", addr, val);
            }
        }
    }

    fn read(&self, addr: u16) -> u8 {
        match addr {
            /* ROM Bank 0 */
            0x0000..=0x3FFF => {
                if self.adv_bank_mode {
                    unimplemented!("Not implementing advanced banking mode!")
                } else {
                    self.rom[addr as usize]
                }
            }

            /* ROM Bank X */
            0x4000..=0x7FFF => {
                let addr = (addr as usize - 0x4000)
                    | (self.rom_bank_num as usize) << 14;

                //TODO On smaller cartridges, the upper bits 
                //     here are ignored, but not always
                    //| (self.ram_bank_num as usize) << 19I;
                self.rom[addr]
            }

            /* RAM Bank X */
            0xA000..=0xBFFF => {
                if !self.ram_en {
                    0xFF
                } else {
                    let offset = (addr - 0xA000) as u32
                        + if self.adv_bank_mode {
                            (self.ram_bank_num as u32) << 13
                        } else {
                            0
                        };
                    self.ram.read(offset)
                }
            }

            _ => {
                unreachable!("Invalid MBC1 address! addr: {:?}", addr)
            }
        }
    }
}
