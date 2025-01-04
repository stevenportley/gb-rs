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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::read;
    use std::path::Path;
    use std::time;

    fn rom_test(rom_path: &Path) {
        let rom = read(rom_path).expect("Unable to load test rom: {rom_path}");
        let rom = Rom::from_slice(rom.as_slice());

        let mut gb = GbRs::new(rom);

        let timeout = time::Instant::now() + time::Duration::from_secs(30);

        let mut cnt = 0;

        while !gb.cpu.is_passed() {
            gb.run_one();

            if cnt == 1000 {
                // Timeout check
                assert!(time::Instant::now() < timeout);
                cnt = 0;
            }

            cnt += 1;
        }
    }

    #[test]
    fn rom1_special() {
        rom_test(Path::new("testroms/testrom-cpuinstr-01.gb"));
    }

    #[test]
    fn rom2_int() {
        rom_test(Path::new("testroms/testrom-cpuinstr-02.gb"));
    }

    #[test]
    fn rom3_op_sp_hl() {
        rom_test(Path::new("testroms/testrom-cpuinstr-03.gb"));
    }

    #[test]
    fn rom4_op_r_imm() {
        rom_test(Path::new("testroms/testrom-cpuinstr-04.gb"));
    }

    #[test]
    fn rom5_op_rp() {
        rom_test(Path::new("testroms/testrom-cpuinstr-05.gb"));
    }

    #[test]
    fn rom6_ld_r_r() {
        rom_test(Path::new("testroms/testrom-cpuinstr-06.gb"));
    }

    #[test]
    fn rom7_jr_jp_call_ret_rst() {
        rom_test(Path::new("testroms/testrom-cpuinstr-07.gb"));
    }

    #[test]
    fn rom8_misc_instr() {
        rom_test(Path::new("testroms/testrom-cpuinstr-08.gb"));
    }

    #[test]
    fn rom9_op_r_r() {
        rom_test(Path::new("testroms/testrom-cpuinstr-09.gb"));
    }

    #[test]
    fn rom10_op_r_r() {
        rom_test(Path::new("testroms/testrom-cpuinstr-10.gb"));
    }

    #[test]
    fn rom11_op_a_hl() {
        rom_test(Path::new("testroms/testrom-cpuinstr-11.gb"));
    }

    #[test]
    fn instr_timing() {
        rom_test(Path::new("testroms/instr_timing.gb"));
    }
}
