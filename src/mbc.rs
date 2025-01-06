use crate::bus::Device;
use core::ops::DerefMut;
use heapless::String;

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

pub trait MemoryRegion: DerefMut<Target = [u8]> {}

pub trait Cartridge {
    type Rom: DerefMut<Target = [u8]> + ?Sized;
    type Ram: DerefMut<Target = [u8]> + ?Sized;

    fn rom(&self) -> &Self::Rom;
    fn rom_mut(&mut self) -> &mut Self::Rom;
    fn ram(&self) -> &Self::Ram;
    fn ram_mut(&mut self) -> &mut Self::Ram;
}

pub enum MemoryBankController<Cart: Cartridge> {
    MBC0(Cart),
    MBC1(MBC1<Cart>),
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

    CartridgeHeader {
        title,
        manufacturer_code,
        //gbc_flag,
        licensee_code: String::new(),
        is_sgb: rom[0x146] != 0x03,
        cart_type: rom[0x147],
        rom_size,
        ram_size,
    }
}

impl<Cart: Cartridge> Device for MemoryBankController<Cart> {
    fn write(&mut self, addr: u16, val: u8) {
        match self {
            MemoryBankController::MBC0(_cart) => { /* NOP */ }
            MemoryBankController::MBC1(mbc) => mbc.write(addr, val),
        }
    }

    fn read(&self, addr: u16) -> u8 {
        match self {
            MemoryBankController::MBC0(cart) => cart.rom()[addr as usize],
            MemoryBankController::MBC1(mbc) => mbc.read(addr),
        }
    }
}

impl<Cart: Cartridge> MemoryBankController<Cart> {
    pub fn new(cart: Cart) -> Self {
        let header = get_cart_header(&cart.rom());

        match header.cart_type {
            0 => MemoryBankController::MBC0(cart),
            1 | 2 | 3 => MemoryBankController::MBC1(MBC1::new(cart)),
            _ => unimplemented!(
                "MBC type not implemented! Cart type: {:?}",
                header.cart_type
            ),
        }
    }

    pub fn get_header(&self) -> CartridgeHeader {
        match self {
            MemoryBankController::MBC0(cart) => get_cart_header(&cart.rom()),
            MemoryBankController::MBC1(mbc) => get_cart_header(&mbc.cart.rom()),
        }
    }
}

struct MBC1<Cart: Cartridge> {
    cart: Cart,
    ram_en: bool,
    rom_bank_num: u8,
    adv_bank_mode: bool,
    ram_bank_num: u8,
}

impl<Cart: Cartridge> MBC1<Cart> {
    fn new(cart: Cart) -> Self {
        Self {
            cart,
            ram_en: false,
            rom_bank_num: 1,
            adv_bank_mode: false,
            ram_bank_num: 1,
        }
    }
}

impl<Cart: Cartridge> Device for MBC1<Cart> {
    fn write(&mut self, addr: u16, val: u8) {
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
            }
            0x4000..=0x5FFF => self.ram_bank_num = val & 0x3,
            0x6000..=0x7FFF => {
                unimplemented!("Not implementing advanced banking mode!");
                self.adv_bank_mode = val & 0x1 == 0x1
            }

            /* Memory banks */
            0xA000..=0xBFFF => {
                if !self.ram_en {
                    // Ignore writes to disabled RAM
                    return;
                }

                let offset = (addr - 0xA000) as u32
                    + if self.adv_bank_mode {
                        (self.ram_bank_num as u32) << 13
                    } else {
                        0
                    };

                //TODO: Size check
                self.cart.ram_mut()[offset as usize] = val;
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
                    self.cart.rom()[addr as usize]
                }
            }

            /* ROM Bank X */
            0x4000..=0x7FFF => {
                let addr = (addr as usize - 0x4000) | (self.rom_bank_num as usize) << 14;

                //TODO On smaller cartridges, the upper bits
                //     here are ignored, but not always
                //| (self.ram_bank_num as usize) << 19I;
                self.cart.rom()[addr]
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
                    self.cart.ram()[offset as usize]
                }
            }

            _ => {
                unreachable!("Invalid MBC1 address! addr: {:?}", addr)
            }
        }
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
