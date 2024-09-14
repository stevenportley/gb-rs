use crate::tile::{Tile, NTILES};

const VRAM_LEN: usize = 0x2000;

pub struct PPU {
    pub vram: [u8; VRAM_LEN],
}

impl PPU {
    pub fn new() -> Self {
        Self {
            vram: [0; VRAM_LEN],
        }
    }

    pub fn dump_vram(&self) -> Vec<Tile> {
        let mut tiles: Vec<Tile> = Vec::with_capacity(NTILES);

        let mut tile_iter = self.vram.chunks(16);

        while let Some(tile) = tile_iter.next() {
            tiles.push(Tile::from_bytes(tile))
        }

        return tiles;
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn base_test() {
        let raw = [
            0x7C, 0x7C, 0x00, 0xC6, 0xC6, 0x00, 0x00, 0xFE, 0xC6, 0xC6, 0x00, 0xC6, 0xC6, 0x00,
            0x00, 0x00,
        ];

        let this_tile = Tile::from_bytes(&raw);

        let exp_tile = Tile {
            pixels: [
                [0, 3, 3, 3, 3, 3, 0, 0],
                [2, 2, 0, 0, 0, 2, 2, 0],
                [1, 1, 0, 0, 0, 1, 1, 0],
                [2, 2, 2, 2, 2, 2, 2, 0],
                [3, 3, 0, 0, 0, 3, 3, 0],
                [2, 2, 0, 0, 0, 2, 2, 0],
                [1, 1, 0, 0, 0, 1, 1, 0],
                [0, 0, 0, 0, 0, 0, 0, 0],
            ],
        };

        assert_eq!(this_tile.pixels, exp_tile.pixels);
    }

    #[test]
    fn base_test2() {
        use std::path::Path;
        use std::fs::read;

        let test_dump = Path::new("roms/bgbtest.dump");
        let rom = read(test_dump).expect("Unable to load test rom: {rom_path}");
        println!("Rom len: {}", rom.len());
        let mut ppu = PPU { vram: [0; VRAM_LEN] };

        ppu.vram.copy_from_slice(&rom);

        let data = ppu.dump_vram();

        for tile in data {
            println!("Tile: {:?}", tile);
        }
    }
}
