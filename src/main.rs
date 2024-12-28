use gb_rs::gb::GbRs;
use gb_rs::rom::Cartridge;
use gb_rs::rom::Rom;

//mod gui;
mod tui;

fn main() -> std::io::Result<()> {
    //let gb = GbRs::new(Rom::acid_cart());
    //let rom_path = std::path::Path::new("roms/tetris.gb");
    //let rom_path = std::path::Path::new("roms/dmg-acid2.gb");
    let rom_path = std::path::Path::new("roms/tennis.gb");
    let rom = std::fs::read(rom_path).expect("Unable to load test rom: {rom_path}");
    let rom = Rom::from_slice(&rom.as_slice()[0..0x8000]);

    let gb = GbRs::new(rom);

    tui::run_tui(gb)?;
    /*
    use crate::gui::Gui;
    let gui = Gui::new(gb);
    gui.run();
    */

    Ok(())
}
