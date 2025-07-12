use zerocopy::FromBytes;
use zerocopy_derive::{FromBytes, Immutable, KnownLayout};


#[derive(Clone, Copy)]
pub struct Palette(pub u8);

impl Palette {
    const DEFAULT_PALETTE: Self = Palette(0b11100100_u8);
}

#[derive(FromBytes, Immutable, KnownLayout)]
pub struct Line {
    data: [u8; 2],
}

impl Line {

    #[inline(always)]
    pub fn apply_palette(color_id: u8, palette: Palette) -> u8 {
        return (palette.0 >> (2 * color_id)) & 0x3;
    }
    
    pub fn render(&self, dest: &mut [u8], palette: Palette) {
        assert_eq!(dest.len(), 8);
        let b1 = self.data[0];
        let b2 = self.data[1];

            dest[0] = ((b2 >> 6) & 0x2) + ((b1 >> 7) & 1);
            dest[1] = ((b2 >> 5) & 0x2) + ((b1 >> 6) & 1);
            dest[2] = ((b2 >> 4) & 0x2) + ((b1 >> 5) & 1);
            dest[3] = ((b2 >> 3) & 0x2) + ((b1 >> 4) & 1);
            dest[4] = ((b2 >> 2) & 0x2) + ((b1 >> 3) & 1);
            dest[5] = ((b2 >> 1) & 0x2) + ((b1 >> 2) & 1);
            dest[6] = ((b2 >> 0) & 0x2) + ((b1 >> 1) & 1);
            dest[7] = ((b2 << 1) & 0x2) + ((b1 >> 0) & 1);

    }

    pub fn partial_render(&self, dest: &mut [u8], palette: Palette) {
        let d_iter = dest.iter_mut().take(8);
        let mut idx = 7;

        let b1 = self.data[0];
        let b2 = self.data[1];

        for d in d_iter {
            // The corresponding bit in each byte that make
            // up the 2 index
            let _b2 = b2.checked_shr(idx - 1).unwrap_or(0) & 0x2;
            let _b1 = b1.checked_shr(idx).unwrap_or(0) & 0x1;
            let color_id = _b2 + _b1;
            *d = Self::apply_palette(color_id, palette);
            idx -= 1;
        }


    }

}


#[derive(FromBytes, Immutable, KnownLayout)]
pub struct Tile {
    pub lines: [Line; 8],
}

impl Tile {
    pub fn render_with_palette(&self, palette: Palette) -> [[u8; 8]; 8] {
        let mut tile = [[0;8]; 8];

        for i in 0..8 {
            self.lines[i].render(&mut tile[i], palette);
        }

        tile
    }

    pub fn render(&self) -> [[u8; 8]; 8] {
        self.render_with_palette(Palette::DEFAULT_PALETTE)
    }
}

#[derive(FromBytes, Immutable, KnownLayout)]
pub struct VramBank {
    tiles: [Tile; 384],
    tilemap0: [u8; 32 * 32],
    tilemap1: [u8; 32 * 32],
    //oam_map: [u8; 32 * 32],
}


impl VramBank {

    pub fn get_bg_tile(&self, idx: usize, alt_address_mode: bool, high_tile_map: bool) -> &Tile {

        let tile_idx = if high_tile_map {
            self.tilemap1[idx]
        } else {
            self.tilemap0[idx]
        };

        if alt_address_mode {
            //Selet 'blocks' 1 and 2
            let tiles = &self.tiles[128..];
            &tiles[tile_idx.wrapping_add(128) as usize]
        } else {
            &self.tiles[tile_idx as usize]
        }
    }

}
