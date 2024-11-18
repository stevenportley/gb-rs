pub mod bus;
pub mod cpu;
pub mod interrupts;
pub mod ppu;
pub mod tile;
pub mod timer;
pub mod oam;
pub mod gb;


pub fn add(left: usize, right: usize) -> usize {
    left + right
}

