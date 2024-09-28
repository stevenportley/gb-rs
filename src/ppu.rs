use crate::tile::Tile;

// The number of tiles in all of VRAM
pub const NTILES: usize = 384;

// The whole background
pub const NUM_BACKGROUND_TILES: usize = 32 * 32;

const VRAM_LEN: usize = 0x2000;
const OAM_LEN: usize = 0xA0;


pub struct PPU {
    pub vram: [u8; VRAM_LEN],
    pub oam: [u8; OAM_LEN],
}

impl PPU {
    pub fn new() -> Self {
        Self {
            vram: [0; VRAM_LEN],
            oam: [0; OAM_LEN],
        }
    }

    pub fn palette_to_rgba(ind: u8) -> [u8; 4] {
        let val = 255 - (85 * ind);
        return [val, val, val, 0xFF];
    }

    pub fn dump_vram(&self) -> [Tile; NTILES] {

        let tiles : [Tile; NTILES] = core::array::from_fn(|index| { 
            self.from_tile_index(index)
        });

        return tiles;
    }

    pub fn from_tile_index(&self, tile_index: usize) -> Tile {
        let index = tile_index * 16;
        return Tile::from_bytes(&self.vram[index..index+16]);
    }

    pub fn get_background(&self) -> [Tile; NUM_BACKGROUND_TILES] {

        //TODO This assumes tile map starts at
        // 0x9800, should be selected based on LCDC register

        let tiles: [Tile; NUM_BACKGROUND_TILES] = core::array::from_fn(|index| {
            let tilemap_index = index + 0x9C00 - 0x8000;
            let tile_index = self.vram[tilemap_index];
            self.from_tile_index(tile_index as usize)
        });

        tiles
    }

}
