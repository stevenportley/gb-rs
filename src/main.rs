use std::env;
use std::io;
use gb_rs::cpu::Cpu;

use std::path::Path;

fn main() -> io::Result<()> {

    let args: Vec<String> = env::args().collect();

    let path = if args.len() != 2 {
        Path::new("roms/testrom-cpuinstr-04.gb")
    } else {
        Path::new(&args[1])
    };

    let rom = std::fs::read(path).expect("Unable to load rom file");
    let mut cpu = Cpu::new(rom.as_slice())?;

    loop {

        cpu.log_state();
        let next_instr = cpu.next_instr();
        let clks = cpu.execute_instr(next_instr);
        if clks == 0 {
            break;
        }

        if cpu.is_passed() {
            break;
        }

    }

    Ok(())
}
