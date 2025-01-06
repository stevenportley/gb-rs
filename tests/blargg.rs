use std::fs::read;
use std::path::Path;
use std::time;

use gb_rs::gb::{GbRs, SmallInMemoryCartridge};

fn rom_test(rom_path: &str) {
    let rom_path = Path::new(rom_path);
    let rom = read(rom_path).expect(format!("Unable to load test rom: {:?}", rom_path).as_str());
    let cartridge = SmallInMemoryCartridge::from_slice(rom.as_slice());

    let mut gb = GbRs::new(cartridge);

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
    rom_test("tests/roms/blargg/testrom-cpuinstr-01.gb");
}

#[test]
fn rom2_int() {
    rom_test("tests/roms/blargg/testrom-cpuinstr-02.gb");
}

#[test]
fn rom3_op_sp_hl() {
    rom_test("tests/roms/blargg/testrom-cpuinstr-03.gb");
}

#[test]
fn rom4_op_r_imm() {
    rom_test("tests/roms/blargg/testrom-cpuinstr-04.gb");
}

#[test]
fn rom5_op_rp() {
    rom_test("tests/roms/blargg/testrom-cpuinstr-05.gb");
}

#[test]
fn rom6_ld_r_r() {
    rom_test("tests/roms/blargg/testrom-cpuinstr-06.gb");
}

#[test]
fn rom7_jr_jp_call_ret_rst() {
    rom_test("tests/roms/blargg/testrom-cpuinstr-07.gb");
}

#[test]
fn rom8_misc_instr() {
    rom_test("tests/roms/blargg/testrom-cpuinstr-08.gb");
}

#[test]
fn rom9_op_r_r() {
    rom_test("tests/roms/blargg/testrom-cpuinstr-09.gb");
}

#[test]
fn rom10_op_r_r() {
    rom_test("tests/roms/blargg/testrom-cpuinstr-10.gb");
}

#[test]
fn rom11_op_a_hl() {
    rom_test("tests/roms/blargg/testrom-cpuinstr-11.gb");
}

#[test]
fn instr_timing() {
    rom_test("tests/roms/blargg/instr_timing.gb");
}
