use core::ops::DerefMut;
use core::time::Duration;
use heapless::String;

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

#[derive(PartialEq, Default)]
struct Mbc1Reg {
    two_bit_reg: u8,
    bank_mode_sel: bool,
}

#[derive(PartialEq)]
enum RamOrRtc {
    BankNum(u8),
    RTC,
}

impl Default for RamOrRtc {
    fn default() -> Self {
        Self::BankNum(0)
    }
}

#[derive(PartialEq, Default)]
struct Mbc3Reg {
    ram_or_rtc: RamOrRtc,
    latch_clock_data: u8,
    rtc: Duration,
}

#[derive(PartialEq)]
enum MemoryBankController {
    MBC0,
    MBC1(Mbc1Reg),
    MBC3(Mbc3Reg),
}

pub struct Cartridge<T: CartridgeData> {
    data: T,
    mbc: MemoryBankController,
    ram_en: bool,
    rom_bank_num: u8,
}

impl<T: CartridgeData> Cartridge<T> {
    pub fn new(data: T) -> Self {
        let header = data.get_header();

        let mbc: MemoryBankController = match header.cart_type {
            0 => MemoryBankController::MBC0,
            1 | 2 | 3 => MemoryBankController::MBC1(Mbc1Reg::default()),
            0x0F..=0x13 => MemoryBankController::MBC3(Mbc3Reg::default()),
            _ => {
                unimplemented!("Unimplemented MBC type")
            }
        };

        Self {
            data,
            mbc,
            ram_en: false,
            rom_bank_num: 1,
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        if self.mbc == MemoryBankController::MBC0 {
            return;
        }

        match addr {
            /* Registers */
            0..=0x1FFF => {
                if (val & 0xF) == 0xA {
                    self.ram_en = true
                } else {
                    self.ram_en = false
                }
            }
            0x2000..=0x3FFF => {
                let mask = match self.mbc {
                    MemoryBankController::MBC0 => {
                        unreachable!("")
                    }
                    MemoryBankController::MBC1(_) => 0x1F,
                    MemoryBankController::MBC3(_) => 0x7F,
                };

                self.rom_bank_num = val & mask;

                // From pandocs:
                // "If the main 5-bit ROM banking register is 0, it reads the bank as if it was set to 1."
                if self.rom_bank_num == 0 {
                    self.rom_bank_num = 1;
                }

                // According to pandocs:
                // "If the ROM Bank Number is set to a higher value than the number of banks in the cart,
                // the bank number is masked to the required number of bits.
                // e.g. a 256 KiB cart only needs a 4-bit bank number to address all of its 16 banks,
                // so this register is masked to 4 bits. The upper bit would be ignored for bank selection."
                //
                // This generates that mask

                let max_banks = self.get_header().num_rom_banks;
                let bank_mask = (max_banks - 1) as u8;
                //let bank_mask = (1 << max_banks.ilog2()) - 1;

                //Note: By performing the masking after the 0 -> 1 translation
                //      above, we satisfy this section of pandocs for MBC1:
                //
                //      "Even with smaller ROMs that use less than 5 bits for bank selection,
                //      the full 5-bit register is still compared for the bank 00→01 translation logic.
                //      As a result if the ROM is 256 KiB or smaller, it is possible to map
                //      bank 0 to the 4000–7FFF region — by setting the 5th bit to 1 it will
                //      prevent the 00→01 translation (which looks at the full 5-bit register, and sees
                //      the value $10, not $00), while the bits actually used for bank selection
                //      (4, in this example) are all 0, so bank $00 is selected."
                self.rom_bank_num = self.rom_bank_num & bank_mask;
            }
            0x4000..=0x5FFF => {
                let ram_size = self.get_header().ram_size;
                let num_rom_banks = self.get_header().num_rom_banks;

                match &mut self.mbc {
                    MemoryBankController::MBC0 => {}
                    MemoryBankController::MBC1(reg) => {
                        let Mbc1Reg { two_bit_reg, .. } = reg;

                        if num_rom_banks >= 64 || ram_size >= 32767 {
                            *two_bit_reg = val & 0x3;
                        }
                    }

                    MemoryBankController::MBC3(regs) => {
                        let Mbc3Reg { ram_or_rtc, .. } = regs;

                        match val {
                            0..=0x3 => {
                                *ram_or_rtc = RamOrRtc::BankNum(val);
                            }
                            0x8..0xC => *ram_or_rtc = RamOrRtc::RTC,
                            _ => { /* No OP */ }
                        }
                    }
                }
            }

            0x6000..=0x7FFF => {
                match &mut self.mbc {
                    MemoryBankController::MBC0 => {}
                    MemoryBankController::MBC1(reg) => {
                        let Mbc1Reg { bank_mode_sel, .. } = reg;
                        *bank_mode_sel = val & 0x1 == 0x1;
                    }
                    MemoryBankController::MBC3(reg) => {
                        let Mbc3Reg {
                            latch_clock_data,
                            rtc,
                            ..
                        } = reg;
                        if *latch_clock_data == 0 && val == 1 {
                            *rtc += Duration::from_millis(1);
                        }
                        *latch_clock_data = val;
                        //panic!("Not implemented!");
                        //TODO: Latch clock data
                    }
                }
            }
            /* Memory banks */
            0xA000..=0xBFFF => {
                if !self.ram_en {
                    // Ignore writes to disabled RAM
                    return;
                }

                match &self.mbc {
                    MemoryBankController::MBC0 => {
                        panic!("Accessing RAM when it doesn't exist!")
                    }
                    MemoryBankController::MBC1(reg) => {
                        let Mbc1Reg {
                            two_bit_reg,
                            bank_mode_sel,
                        } = reg;

                        let mut addr = (addr - 0xA000) as usize;

                        if *bank_mode_sel {
                            addr |= (*two_bit_reg as usize) << 13;
                        }

                        self.data.ram_mut()[addr] = val;
                    }

                    MemoryBankController::MBC3(reg) => {
                        let Mbc3Reg { ram_or_rtc, .. } = reg;
                        let mut addr = (addr - 0xA000) as usize;
                        match ram_or_rtc {
                            RamOrRtc::RTC => { /* TODO, How does this work?? */ }
                            RamOrRtc::BankNum(bank) => {
                                addr |= (*bank as usize) << 13;
                            }
                        }
                        //TODO: Size check
                        if addr < (self.get_header().ram_size as usize) {
                            self.data.ram_mut()[addr] = val;
                        }
                    }
                }
            }
            _ => {
                unreachable!("Invalid MBC1 address! addr: {:?}, val: {:?}", addr, val);
            }
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        if self.mbc == MemoryBankController::MBC0 {
            return self.data.rom()[addr as usize];
        }

        match addr {
            /* ROM Bank 0 */
            0x0000..=0x3FFF => {
                let mut addr = addr as usize;

                if let MemoryBankController::MBC1(regs) = &self.mbc {
                    if regs.bank_mode_sel {
                        addr |= (regs.two_bit_reg as usize) << 19;
                    }
                }

                let mut mask = 1 << 20;
                while addr >= self.get_header().rom_size as usize {
                    addr &= !mask;
                    mask >>= 1;
                }

                return self.data.rom()[addr as usize];
            }

            /* ROM Bank X */
            0x4000..=0x7FFF => {
                let mut addr = addr as usize - 0x4000;

                addr |= (self.rom_bank_num as usize) << 14;
                if let MemoryBankController::MBC1(regs) = &self.mbc {
                    addr |= (regs.two_bit_reg as usize) << 19;
                }

                let mut mask = 1 << 20;
                while addr > self.get_header().rom_size as usize {
                    addr &= !mask;
                    mask >>= 1;
                }

                return self.data.rom()[addr];
            }

            /* RAM Bank X */
            0xA000..=0xBFFF => {
                if !self.ram_en {
                    // Ignore writes to disabled RAM
                    return 0xFF;
                }

                let mut addr = (addr - 0xA000) as usize;

                if let MemoryBankController::MBC3(regs) = &self.mbc {

                    match regs.ram_or_rtc {
                        RamOrRtc::RTC => { return 0; /* TODO: RTC */ },
                        RamOrRtc::BankNum(bank) => {
                            addr |= (bank as usize) << 13;
                        }

                    }
                }

                if let MemoryBankController::MBC1(regs) = &self.mbc {
                    if regs.bank_mode_sel {
                        addr |= (regs.two_bit_reg as usize) << 13;
                    }
                }

                //TODO: Size check
                self.data.ram()[addr]
            }

            _ => {
                unreachable!("Invalid MBC1 address! addr: {:?}", addr)
            }
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
    let title = String::from_utf8(title).unwrap_or(String::new()); //("The title is invalid UTF-8");

    let manufacturer_code = (0x13F..=0x143)
        .into_iter()
        .map(|addr| rom[addr])
        .take_while(|b| *b != 0)
        .collect();
    let manufacturer_code = String::from_utf8(manufacturer_code).unwrap_or(String::new()); //expect("The manufacturer is invalid UTF-8");

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
