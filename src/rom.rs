
const ROM_LEN: usize = 0x4000;

pub struct Rom {
    rom: [u8; ROM_LEN],
    mapped_rom: [u8; ROM_LEN],
}

impl Rom {

    pub fn tetris_cart() -> Rom {
        const BYTES: &[u8; 2 * ROM_LEN] = include_bytes!("../roms/tetris.gb");
        let rom = Rom { 
            rom: BYTES[..ROM_LEN].try_into().unwrap(), 
            mapped_rom: BYTES[ROM_LEN..].try_into().unwrap(),
        };
        rom
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
        if addr < self.rom.len() {
            self.rom[addr]
        } else {
            self.mapped_rom[addr - self.rom.len()]
        }
    }

}
