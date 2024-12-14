use gb_rs::gb::GbRs;
use gb_rs::rom::Rom;

mod gui;
mod tui;



fn main() -> std::io::Result<()> {
    let gb = GbRs::new(Rom::tetris_cart());

    tui::run_tui(gb)?;
    /*
    use crate::gui::Gui;
    let gui = Gui::new(gb);
    gui.run();
    */

    Ok(())
}
