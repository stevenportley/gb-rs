const ROM_LEN: usize = 0x4000;
const TETRIS: &[u8; 2 * ROM_LEN] = include_bytes!("../roms/tetris.gb");
const ACID: &[u8; 2 * ROM_LEN] = include_bytes!("../roms/dmg-acid2.gb");

pub struct Rom {
    rom: [u8; ROM_LEN],
    mapped_rom: [u8; ROM_LEN],
}

impl Rom {
    pub fn tetris_cart() -> Rom {
        Self {
            rom: TETRIS[..ROM_LEN].try_into().unwrap(),
            mapped_rom: TETRIS[ROM_LEN..].try_into().unwrap(),
        }
    }

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

    pub fn read(&self, addr: u16) -> u8 {
        let addr = addr as usize;
        if addr < ROM_LEN {
            self.rom[addr]
        } else {
            self.mapped_rom[addr - ROM_LEN]
        }
    }
}
