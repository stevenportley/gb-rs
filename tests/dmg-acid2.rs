use gb_rs::gb::{GbRs, SmallInMemoryCartridge};
use std::fs::read;
use std::path::Path;

#[test]
fn dmg2_acid_test() {
    let rom_path = Path::new("tests/roms/dmg-acid2.gb");
    let bin_path = Path::new("tests/dmg-acid2.bin");

    let rom = read(rom_path).expect("Unable to load dmg-acid2 ROM");
    let bin = read(bin_path).expect("Unable to load dmg-acid2 Golden reference.");

    let cartridge = SmallInMemoryCartridge::from_slice(rom.as_slice());

    let mut gb = GbRs::new(cartridge);

    for _ in 0..10 {
        gb.run_frame();
    }

    assert_eq!(gb.cpu.bus.ppu.get_screen(), *bin);
}
