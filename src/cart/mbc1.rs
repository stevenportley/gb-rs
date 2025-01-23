use crate::cart::CartridgeData;

pub struct MBC1 {
    ram_en: bool,
    rom_bank_num: u8,
    secondary_bank_reg: u8,
    adv_bank_mode: bool,
}

impl MBC1 {
    pub fn new() -> Self {
        Self {
            ram_en: false,
            rom_bank_num: 1,
            secondary_bank_reg: 0,
            adv_bank_mode: false,
        }
    }

    pub fn write<Cart: CartridgeData>(&mut self, cart: &mut Cart, addr: u16, val: u8) {
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
                self.rom_bank_num = val & 0x1F;

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

                let max_banks = cart.get_header().num_rom_banks;
                let bank_mask = (1 << max_banks.ilog2()) - 1;

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

                let ram_size = cart.get_header().ram_size;
                let num_rom_banks = cart.get_header().num_rom_banks;

                //assert_eq!(num_rom_banks, 64);

                if num_rom_banks >= 64 || ram_size >= 32767 {
                    self.secondary_bank_reg = val & 0x3;
                }

            }
            0x6000..=0x7FFF => {
                //unimplemented!("Not implementing advanced banking mode!");
                self.adv_bank_mode = val & 0x1 == 0x1;
            }

            /* Memory banks */
            0xA000..=0xBFFF => {
                if !self.ram_en {
                    // Ignore writes to disabled RAM
                    return;
                }

                let mut offset = (addr - 0xA000) as u32;

                if self.adv_bank_mode {
                    offset |= (self.secondary_bank_reg as u32) << 13;
                }

                //TODO: Size check
                cart.ram_mut()[offset as usize] = val;
            }
            _ => {
                unreachable!("Invalid MBC1 address! addr: {:?}, val: {:?}", addr, val);
            }
        }
    }

    pub fn read<Cart: CartridgeData>(&self, cart: &Cart, addr: u16) -> u8 {
        match addr {
            /* ROM Bank 0 */
            0x0000..=0x3FFF => {

                let mut addr = addr as usize;

                if self.adv_bank_mode {
                    addr |= (self.secondary_bank_reg as usize) << 19;
                }

                let mut mask = 1 << 20;
                while addr >= cart.get_header().rom_size as usize {
                    addr &= !mask;
                    mask >>= 1;
                }

                cart.rom()[addr as usize]
            }

            /* ROM Bank X */
            0x4000..=0x7FFF => {

                let mut addr = addr as usize - 0x4000;
                addr |= (self.rom_bank_num as usize) << 14;

                assert!(self.rom_bank_num < 32);
                addr |= (self.secondary_bank_reg as usize) << 19;

                let mut mask = 1 << 20;
                while addr > cart.get_header().rom_size as usize {
                    addr &= !mask;
                    mask >>= 1;
                }

                cart.rom()[addr]
            }

            /* RAM Bank X */
            0xA000..=0xBFFF => {

                if !self.ram_en {
                    // Ignore writes to disabled RAM
                    return 0xFF;
                }

                let mut offset = (addr - 0xA000) as u32;

                if self.adv_bank_mode {
                    offset += (self.secondary_bank_reg as u32) << 13;
                }

                //TODO: Size check
                cart.ram()[offset as usize]
            }

            _ => {
                unreachable!("Invalid MBC1 address! addr: {:?}", addr)
            }
        }
    }
}
