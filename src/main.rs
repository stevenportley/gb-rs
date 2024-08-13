use std::env;
use gb_rs::cpu::Cpu;
use std::fs::read;

use std::path::Path;

fn main() {

    let args: Vec<String> = env::args().collect();

    let path = if args.len() != 2 {
        Path::new("roms/testrom-cpuinstr-04.gb")
    } else {
        Path::new(&args[1])
    };


    let mut cpu = Cpu::new();


    let rom = std::fs::read(path).expect("Unable to load rom file");

    cpu.memory[0..32768].copy_from_slice(&rom);
    cpu.memory[0xFF44] = 0x90;

    loop {

        cpu.log_state();
        let next_instr = cpu.next_instr();
        let clks = cpu.execute_instr(next_instr);
        if clks == 0 {
            break;
        }


    }



}
