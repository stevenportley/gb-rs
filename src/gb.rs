use crate::cpu::Cpu;
use crate::interrupts::IntSource;
use std::io;

pub struct GbRs {
    pub cpu: Cpu,
    clk_cnter: i32,
}


impl GbRs {
    pub fn new<T: io::Read>(rom: T) -> io::Result<Self> {
        Ok(Self {
            cpu: Cpu::new(rom)?,
            clk_cnter: 0,
        })
    }


    pub fn run_one(&mut self) {

        let (ppu_cycles, maybe_int) = self.cpu.bus.ppu.run_one();

        self.clk_cnter += ppu_cycles;

        while self.clk_cnter >= 0 {
            let cpu_cycles = self.cpu.run_one();

            for _ in 0..cpu_cycles {

                if self.cpu.bus.timer.tick() {
                    self.cpu.bus.int_controller.interrupt(IntSource::TIMER);
                }
            }

            self.clk_cnter -= cpu_cycles as i32;
        }

        if let Some(ppu_int) = maybe_int {
            self.cpu.bus.int_controller.interrupt(ppu_int)
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

        let mut gb = GbRs::new(rom.as_slice()).expect("Unable to load test rom");

        let timeout = time::Instant::now() + time::Duration::from_secs(5);

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
        rom_test(Path::new("roms/testrom-cpuinstr-01.gb"));
    }

    #[test]
    fn rom2_int() {
        rom_test(Path::new("roms/testrom-cpuinstr-02.gb"));
    }

    #[test]
    fn rom3_op_sp_hl() {
        rom_test(Path::new("roms/testrom-cpuinstr-03.gb"));
    }

    #[test]
    fn rom4_op_r_imm() {
        rom_test(Path::new("roms/testrom-cpuinstr-04.gb"));
    }

    #[test]
    fn rom5_op_rp() {
        rom_test(Path::new("roms/testrom-cpuinstr-05.gb"));
    }

    #[test]
    fn rom6_ld_r_r() {
        rom_test(Path::new("roms/testrom-cpuinstr-06.gb"));
    }

    #[test]
    fn rom7_jr_jp_call_ret_rst() {
        rom_test(Path::new("roms/testrom-cpuinstr-07.gb"));
    }

    #[test]
    fn rom8_misc_instr() {
        rom_test(Path::new("roms/testrom-cpuinstr-08.gb"));
    }

    #[test]
    fn rom9_op_r_r() {
        rom_test(Path::new("roms/testrom-cpuinstr-09.gb"));
    }

    #[test]
    fn rom10_op_r_r() {
        rom_test(Path::new("roms/testrom-cpuinstr-10.gb"));
    }

    #[test]
    fn rom11_op_a_hl() {
        rom_test(Path::new("roms/testrom-cpuinstr-11.gb"));
    }

    #[test]
    fn instr_timing() {
        rom_test(Path::new("roms/instr_timing.gb"));
    }
}
