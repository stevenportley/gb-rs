use std::io;

pub struct Bus {
    rom: [u8; 0x4000],
    mapped_rom: [u8; 0x4000], //TODO(SP): This needs to be changed to support ROM bank mapper
    vram: [u8; 0x2000],
    eram: [u8; 0x2000],
    wram: [u8; 0x1000],
    mapped_wram: [u8; 0x1000],
    oam: [u8; 0xA0],
    io: [u8; 0x80],
    hram: [u8; 0x7F],
    ie_reg: u8,
}

impl Bus {
    pub fn new<T: io::Read>(mut rom: T) -> io::Result<Self> {
        let mut bus = Self {
            rom: [0; 0x4000],
            mapped_rom: [0; 0x4000],
            vram: [0; 0x2000],
            eram: [0; 0x2000],
            wram: [0; 0x1000],
            mapped_wram: [0; 0x1000],
            oam: [0; 0xA0],
            io: [0; 0x80],
            hram: [0; 0x7F],
            ie_reg: 0,
        };

        rom.read_exact(&mut bus.rom)?;
        rom.read_exact(&mut bus.mapped_rom)?;
        Ok(bus)
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0..=0x3FFF => {
                self.rom[addr as usize] = val;
            }
            0x4000..=0x7FFF => {
                self.mapped_rom[addr as usize - 0x4000] = val;
            }
            0x8000..=0x9FFF => {
                self.vram[addr as usize - 0x8000] = val;
            }
            0xA000..=0xBFFF => {
                self.eram[addr as usize - 0xA000] = val;
            }
            0xC000..=0xCFFF => {
                self.wram[addr as usize - 0xC000] = val;
            }
            0xD000..=0xDFFF => {
                self.mapped_wram[addr as usize - 0xD000] = val;
            }
            0xE000..=0xFDFF => {
                unreachable!("Attempting to write to echo ram! {addr}, {val}");
            }
            0xFE00..=0xFE9F => {
                self.oam[addr as usize - 0xFE00] = val;
            }
            0xFEA0..=0xFEFF => {
                unreachable!("Attempting to write to echo ram! {addr}, {val}");
            }
            0xFF00..=0xFF7F => {
                self.io[addr as usize - 0xFF00] = val;
            }
            0xFF80..=0xFFFe => {
                self.hram[addr as usize - 0xFF80] = val;
            }
            0xFFFF => {
                self.ie_reg = val;
            }
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0..=0x3FFF => {
                return self.rom[addr as usize];
            }
            0x4000..=0x7FFF => {
                return self.mapped_rom[addr as usize - 0x4000];
            }
            0x8000..=0x9FFF => {
                return self.vram[addr as usize - 0x8000];
            }
            0xA000..=0xBFFF => {
                return self.eram[addr as usize - 0xA000];
            }
            0xC000..=0xCFFF => {
                return self.wram[addr as usize - 0xC000];
            }
            0xD000..=0xDFFF => {
                return self.mapped_wram[addr as usize - 0xD000];
            }
            0xE000..=0xFDFF => {
                unreachable!("Attempting to read from echo ram! {addr}");
            }
            0xFE00..=0xFE9F => {
                return self.oam[addr as usize - 0xFE00];
            }
            0xFEA0..=0xFEFF => {
                unreachable!("Attempting to read from echo ram! {addr}");
            }
            0xFF00..=0xFF7F => {
                return self.io[addr as usize - 0xFF00];
            }
            0xFF80..=0xFFFe => {
                return self.hram[addr as usize - 0xFF80];
            }
            0xFFFF => {
                return self.ie_reg;
            }
        }
    }
}
