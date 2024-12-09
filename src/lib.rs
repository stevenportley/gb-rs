pub mod bus;
pub mod cpu;
pub mod gb;
pub mod interrupts;
pub mod oam;
pub mod ppu;
pub mod tile;
pub mod timer;
pub mod joypad;

pub fn add(left: usize, right: usize) -> usize {
    left + right
}
