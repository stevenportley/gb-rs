use crate::bus::StaticBus;
use crate::cpu::Cpu;
use crate::ppu::SCREEN_HEIGHT;
use crate::rom::Rom;

const CYCLES_PER_FRAME: i32 = 17556;

pub struct GbRs {
    pub cpu: Cpu<StaticBus>,
}

impl GbRs {
    pub fn new(rom: Rom) -> Self {
        Self {
            cpu: Cpu::new(StaticBus::new(rom)),
        }
    }

    pub fn run_one(&mut self) -> usize {
        self.cpu.run_one()
    }

    pub fn run_line(&mut self) {
        // Cycles per line
        let mut cyc_remaining: i32 = CYCLES_PER_FRAME / SCREEN_HEIGHT as i32;
        while cyc_remaining > 0 {
            cyc_remaining -= self.run_one() as i32;
        }
    }

    pub fn run_frame(&mut self) {
        let mut cyc_remaining: i32 = CYCLES_PER_FRAME;
        while cyc_remaining > 0 {
            cyc_remaining -= self.run_one() as i32;
        }
    }
}
