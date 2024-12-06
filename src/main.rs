use gb_rs::gb::GbRs;
use std::env;
use std::io;

mod gui;
mod tui;


use std::path::Path;


fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let path = if args.len() != 2 {
        Path::new("roms/tetris.gb")
    } else {
        Path::new(&args[1])
    };

    let rom = std::fs::read(path).expect("Unable to load rom file");
    let gb = GbRs::new(rom.as_slice())?;

    tui::run_tui(gb)?;
    /*
    use crate::gui::Gui;
    let gui = Gui::new(gb);
    gui.run();
    */

    Ok(())
}
