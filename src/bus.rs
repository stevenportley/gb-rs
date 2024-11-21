use std::collections::VecDeque;
use std::io;

use crate::interrupts::InterruptController;
use crate::ppu::PPU;
use crate::timer::Timer;

pub struct Bus {
    rom: [u8; 0x4000],
    mapped_rom: [u8; 0x4000], //TODO(SP): This needs to be changed to support ROM bank mapper
    pub ppu: PPU,
    eram: [u8; 0x2000],
    wram: [u8; 0x1000],
    mapped_wram: [u8; 0x1000],
    pub timer: Timer,
    pub int_controller: InterruptController,
    io: [u8; 0x80],
    hram: [u8; 0x7F],
    passed_buf: VecDeque<u8>,
}

impl Bus {
    pub fn new<T: io::Read>(mut rom: T) -> io::Result<Self> {
        let mut bus = Self {
            rom: [0; 0x4000],
            mapped_rom: [0; 0x4000],
            ppu: PPU::new(),
            eram: [0; 0x2000],
            wram: [0; 0x1000],
            mapped_wram: [0; 0x1000],
            timer: Timer::new(),
            int_controller: InterruptController::new(),
            io: [0; 0x80],
            hram: [0; 0x7F],
            passed_buf: VecDeque::new(),
        };

        rom.read_exact(&mut bus.rom)?;
        rom.read_exact(&mut bus.mapped_rom)?;
        Ok(bus)
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0..=0x3FFF => {
                println!("Ignoring write to cartridge")
                //self.rom[addr as usize] = val;
            }
            0x4000..=0x7FFF => {
                self.mapped_rom[addr as usize - 0x4000] = val;
            }
            0x8000..=0x9FFF => {
                self.ppu.write(addr, val);
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
                //OAM
                self.ppu.write(addr, val);
            }
            0xFEA0..=0xFEFF => {
                println!("Attempting to write to prohibited area! {addr}, {val}");
            }
            0xFF00..=0xFF03 => {
                self.io[addr as usize - 0xFF00] = val;
                if addr == 0xFF01 {
                    self.passed_buf.push_back(val);
                    if self.passed_buf.len() > 6 {
                        self.passed_buf.pop_front();
                    }
                    self.passed_buf.make_contiguous();
                }
            }
            0xFF04..=0xFF07 => {
                self.timer.write(addr, val);
            }
            0xFF08..=0xFF0E => {
                println!("Attempting to write to prohibited area! {addr}, {val}");
            }
            0xFF0F => {
                self.int_controller.write(addr, val);
            }
            0xFF10..=0xFF3F => {
                self.io[addr as usize - 0xFF00] = val;
            }
            //PPU control registers
            0xFF40..=0xFF4B => {
                if addr == 0xFF46 {
                    let mut src = val as u16 * 0x100;
                    for dst in 0xFE00..=0xFE9F {
                        self.write(dst, self.read(src));
                        src += 1;
                    }
                } else {
                    self.ppu.write(addr, val);
                }
            }
            0xFF4C..=0xFF7F => {
                self.io[addr as usize - 0xFF00] = val;
            }
            0xFF80..=0xFFFe => {
                self.hram[addr as usize - 0xFF80] = val;
            }
            0xFFFF => {
                self.int_controller.write(addr, val);
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
                return self.ppu.read(addr);
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
                println!("Attempting to read from echo ram! {addr}");
                return 0;
            }
            0xFE00..=0xFE9F => {
                return self.ppu.read(addr);
            }
            0xFEA0..=0xFEFF => {
                println!("Attempting to read from invalid ram! {addr}");
                return 0;
            }
            0xFF00 => {
                // Joypad input
                return 0xFF;
            }
            0xFF01..=0xFF03 => {
                return self.io[addr as usize - 0xFF00];
            }
            0xFF04..=0xFF07 => {
                return self.timer.read(addr);
            }
            0xFF08..=0xFF0E => {
                println!("Attempting to read from invalid ram! {addr}");
                return 0;
            }
            0xFF0F => {
                return self.int_controller.read(addr);
            }
            0xFF10..=0xFF3F => {
                return self.io[addr as usize - 0xFF00];
            }
            0xFF40..=0xFF4B => {
                // LCD control registers
                return self.ppu.read(addr);
            }
            0xFF4C..=0xFF7F => {
                return self.io[addr as usize - 0xFF00];
            }
            0xFF80..=0xFFFE => {
                return self.hram[addr as usize - 0xFF80];
            }
            0xFFFF => {
                return self.int_controller.read(addr);
            }
        }
    }

    pub fn is_passed(&self) -> bool {
        let (sl, _) = self.passed_buf.as_slices();
        let str = std::str::from_utf8(sl).expect("No!");
        return str == "Passed";
    }
}
