use std::env;
use gb_rs::cpu::Cpu;
use std::fs::read;

use std::path::Path;

fn main() {

    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Expected 1 command line argument for ROM file");
        return;
    }


    let mut cpu = Cpu::new();

    let path = Path::new(&args[1]);

    let rom = std::fs::read(path).expect("Unable to load rom file");

    cpu.memory[0..32768].copy_from_slice(&rom);

    loop {

        cpu.log_state();
        let next_instr = cpu.next_instr();
        let clks = cpu.execute_instr(next_instr);
        if clks == 0 {
            break;
        }


    }



}
