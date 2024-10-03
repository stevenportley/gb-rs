use crate::tile::Tile;

// The number of tiles in all of VRAM
pub const NTILES: usize = 384;

// The whole background
pub const NUM_BACKGROUND_TILES: usize = 32 * 32;

const VRAM_LEN: usize = 0x2000;
const OAM_LEN: usize = 0xA0;

pub enum PpuMode {
    HBLANK,
    OAM_SCAN,
    DRAW,
}

pub struct PPU {
    pub VRAM: [u8; VRAM_LEN],
    OAM: [u8; OAM_LEN],
    LCDC: u8,
    STAT: u8,
    SCY: u8,
    SCX: u8,
    LY: u8,
    LYC: u8,
    BGP: u8,
    OBP0: u8,
    OBP1: u8,
    WY: u8,
    WX: u8,
    curr_x: u8,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            VRAM: [0; VRAM_LEN],
            OAM: [0; OAM_LEN],
            //TODO: Are all of these actually 0 after POR?
            LCDC: 0,
            STAT: 0,
            SCY: 0,
            SCX: 0,
            LY: 0,
            LYC: 0,
            BGP: 0,
            OBP0: 0,
            OBP1: 0,
            WY: 0,
            WX: 0,
            curr_x: 0,
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x8000..=0x9FFF => {
                self.VRAM[addr as usize - 0x8000] = val;
            }
            0xFE00..=0xFE9f => {
                self.OAM[addr as usize - 0xFE00] = val;
            }
            0xFF40 => { self.LCDC = val; }
            0xFF41 => { self.STAT = val; }
            0xFF42 => { self.SCY = val; }
            0xFF43 => { self.SCX = val; }
            0xFF44 => { println!("Trying to write to LY, a read-only register"); }
            0xFF45 => { self.LYC = val; }
            0xFF46 => { unimplemented!("DMA not implemented!") }
            0xFF47 => { self.BGP = val; }
            0xFF48 => { self.OBP0 = val; }
            0xFF49 => { self.OBP1 = val; }
            0xFF4A => { self.WY = val; }
            0xFF4B => { self.WX = val; }
            _ => { unreachable!("Invalid write to PPU? addr:{:?}, val:{:?}", addr, val); }

        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0x9FFF => {
                return self.VRAM[addr as usize - 0x8000];
            }
            0xFE00..=0xFE9f => {
                return self.OAM[addr as usize - 0xFE00];
            }
            0xFF40 => { return self.LCDC; }
            0xFF41 => { return self.STAT; }
            0xFF42 => { return self.SCY; }
            0xFF43 => { return self.SCX; }
            0xFF44 => { return 0x90;/* return self.LY;*/ }
            0xFF45 => { return self.LYC; }
            0xFF46 => { unimplemented!("DMA not implemented!") }
            0xFF47 => { return self.BGP; }
            0xFF48 => { return self.OBP0; }
            0xFF49 => { return self.OBP1; }
            0xFF4A => { return self.WY; }
            0xFF4B => { return self.WX; }
            _ => { unreachable!("Invalid read from PPU? addr:{:?}", addr); }

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
        return Tile::from_bytes(&self.VRAM[index..index+16]);
    }

    pub fn get_background(&self) -> [Tile; NUM_BACKGROUND_TILES] {

        //TODO This assumes tile map starts at
        // 0x9800, should be selected based on LCDC register

        let tiles: [Tile; NUM_BACKGROUND_TILES] = core::array::from_fn(|index| {
            let tilemap_index = index + 0x9800 - 0x8000;
            let tile_index = self.VRAM[tilemap_index];
            self.from_tile_index(tile_index as usize)
        });

        tiles
    }


    pub fn tick(&mut self) -> bool {
        if self.curr_x == 113 {
            self.curr_x = 0;
            if self.LY == 153 {
                self.LY = 0;
                return true;
            } else {
                self.LY += 1;
            }
        } else {
            self.curr_x += 1;
        }

        return false;
    }

}


pub struct PixelFifo<'a> {
    ppu: &'a PPU,
    line_index: usize,



}
