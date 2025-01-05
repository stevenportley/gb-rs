use heapless::Deque;
use heapless::Vec;

use crate::interrupts::{IntSource, InterruptController};
use crate::joypad::Joypad;
use crate::ppu::PPU;
use crate::mbc::MBC;
use crate::timer::Timer;

pub trait Device {
    fn write(&mut self, addr: u16, val: u8);
    fn read(&self, addr: u16) -> u8;
}

#[derive(Default)]
struct BusStats {
    prohibited_area: u16,
    unmapped: u16,
    echo: u16,
}

pub struct Bus {
    pub ppu: PPU,
    wram: [u8; 0x1000],
    mapped_wram: [u8; 0x1000],
    pub timer: Timer,
    pub int_controller: InterruptController,
    pub joypad: Joypad,
    io: [u8; 0x80],
    hram: [u8; 0x7F],
    passed_buf: Deque<u8, 6>,
    stats: BusStats,
    pub rom: MBC<65536>,
}

impl Device for Bus {
    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0..=0x7FFF => {
                self.rom.write(addr, val);
            }
            0x8000..=0x9FFF => {
                self.ppu.write(addr, val);
            }
            0xA000..=0xBFFF => {
                self.rom.write(addr, val);
            }
            0xC000..=0xCFFF => {
                self.wram[addr as usize - 0xC000] = val;
            }
            0xD000..=0xDFFF => {
                self.mapped_wram[addr as usize - 0xD000] = val;
            }
            0xE000..=0xFDFF => {
                self.stats.echo += 1;
            }
            0xFE00..=0xFE9F => {
                //OAM
                self.ppu.write(addr, val);
            }
            0xFEA0..=0xFEFF => {
                self.stats.prohibited_area += 1;
            }
            0xFF00 => {
                self.joypad.write(addr, val);
            }
            0xFF01..=0xFF03 => {
                self.io[addr as usize - 0xFF00] = val;
                if addr == 0xFF01 {
                    if self.passed_buf.is_full() {
                        let _ = self.passed_buf.pop_front();
                    }
                    let _ = self.passed_buf.push_back(val);
                }
            }
            0xFF04..=0xFF07 => {
                self.timer.write(addr, val);
            }
            0xFF08..=0xFF0E => {
                self.stats.unmapped += 1;
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

    fn read(&self, addr: u16) -> u8 {
        match addr {
            0..=0x7FFF => {
                return self.rom.read(addr);
            }
            0x8000..=0x9FFF => {
                return self.ppu.read(addr);
            }
            0xA000..=0xBFFF => {
                self.rom.read(addr)
            }
            0xC000..=0xCFFF => {
                return self.wram[addr as usize - 0xC000];
            }
            0xD000..=0xDFFF => {
                return self.mapped_wram[addr as usize - 0xD000];
            }
            0xE000..=0xFDFF => {
                return 0;
            }
            0xFE00..=0xFE9F => {
                return self.ppu.read(addr);
            }
            0xFEA0..=0xFEFF => {
                return 0;
            }
            0xFF00 => {
                return self.joypad.read(addr);
            }
            0xFF01..=0xFF03 => {
                return self.io[addr as usize - 0xFF00];
            }
            0xFF04..=0xFF07 => {
                return self.timer.read(addr);
            }
            0xFF08..=0xFF0E => {
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
}

impl Bus {
    pub fn new(rom: &[u8]) -> Self {
        Self {
            ppu: PPU::new(),
            wram: [0; 0x1000],
            mapped_wram: [0; 0x1000],
            timer: Timer::new(),
            int_controller: InterruptController::new(),
            joypad: Joypad::new(),
            io: [0; 0x80],
            hram: [0; 0x7F],
            passed_buf: Deque::new(),
            stats: BusStats::default(),
            rom: MBC::new(rom),
        }
    }

    pub fn is_passed(&self) -> bool {

        let buf: Vec<_, 10> = self.passed_buf.clone().into_iter().collect();
        let str = core::str::from_utf8(&buf).expect("No!");

        let moon_passed: [u8; 6] = [3, 5, 8, 13, 21, 34];
        return str == "Passed" || buf.ends_with(&moon_passed);
    }

    pub fn query_interrupt(&mut self) -> Option<IntSource> {
        self.int_controller.next()
    }

    pub fn clear_interrupt(&mut self, interrupt: IntSource) {
        self.int_controller.interrupt_clear(interrupt);
    }

    pub fn run_cycles(&mut self, cycles: u16) {
        /* Move along the PPU */
        let maybe_int = self.ppu.run(cycles as i32);

        /* Move along the timer */
        for _ in 0..cycles {
            if self.timer.tick() {
                self.int_controller.interrupt(IntSource::TIMER);
            }
        }

        /* Handle PPU interrupts */
        // TODO: Why not do this with the `run` call?
        //       immediately?
        if let Some(ppu_int) = maybe_int {
            self.int_controller.interrupt(ppu_int)
        }
    }

    pub fn interrupt_pending(&self) -> bool {
        self.int_controller.pending()
    }
}
