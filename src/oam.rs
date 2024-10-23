use crate::tile::Tile;

pub struct OamEntry<'a> {
    data: &'a [u8],
}

pub struct OamFlags {
    priority: bool,
    y_flip: bool,
    x_flip: bool,
    dmg_palette: bool,
    // These are GB color only
    //bank: bool,
    //cgb_pallete: u8,
}

impl<'a> OamEntry<'a> {
    pub fn from_bytes(data: &'a [u8]) -> Self {
        assert_eq!(data.len(), 4);
        Self { data }
    }

    pub fn y_pos(&self) -> u8 {
        return self.data[0];
    }

    pub fn x_pos(&self) -> u8 {
        return self.data[1];
    }

    pub fn tile_idx(&self) -> u8 {
        return self.data[2];
    }

    pub fn oam_flags(&self) -> OamFlags {
        let flags = self.data[3];
        OamFlags {
            priority: (flags & 0x80 != 0),
            y_flip: (flags & 0x40 != 0),
            x_flip: (flags & 0x20 != 0),
            dmg_palette: (flags & 0x10 != 0),
            //bank: (flags & 0x08 != 0),
        }
    }

    pub fn get_pixels(&self, tiles: &[Tile], mut line_idx: u8) -> [u8; 8] {
        let flags = self.oam_flags();
        let tile_idx = self.tile_idx();
        let tile = &tiles[tile_idx as usize];

        if flags.y_flip {
            //TODO: Assumes 8 pixels in height
            line_idx = 7 - line_idx;
        }

        let mut pixels = tile.pixel_buf(line_idx);

        if flags.x_flip {
            pixels.reverse();
        }

        pixels
    }
}

pub struct OamMap<'a> {
    oam_entries: [OamEntry<'a>; 40],
}

impl<'a> OamMap<'a> {
    pub fn from_mem(data: &'a [u8]) -> Self {
        assert_eq!(data.len(), 0xA0);
        let oam_entries: [OamEntry<'a>; 40] =
            core::array::from_fn(|index| OamEntry::from_bytes(&data[(4 * index)..(4 * index + 4)]));

        Self { oam_entries }
    }

    pub fn iter(&self, ly: u8, large_tiles: bool) -> OamIter {
        OamIter {
            oam_iter: self.oam_entries.iter(),
            ly,
            // TODO: We assume LCDC bit 2
            // denotes only one tile in height,
            // this should actually be read from the register
            tile_height: if large_tiles { 16 } else { 8 },
            cnt: 0,
        }
    }

    pub fn render_line(&self, pixels: &mut [u8],  tiles: &[Tile], ly: u8, large_tiles: bool) -> u32 {
        assert_eq!(pixels.len(), 256);

        let mut oam_iter = self.iter(ly, large_tiles);

        while let Some(oam) = oam_iter.next() {
            let x = oam.x_pos() as usize;

            if x == 0 || x >= 168 {
                // Off the screen
                continue;
            }

            let oam_pixels = oam.get_pixels(tiles, ly % 8);

            if x < 8 {
                // Clipped at beginning of line
                pixels[..x].copy_from_slice(&oam_pixels[8 - x..]);
            } else if x > 160 {
                // Clipped at end of line
                let b = 168 - x;
                pixels[x - 8..].copy_from_slice(&oam_pixels[..b]);
            } else {
                pixels[x - 8..x].copy_from_slice(&oam_pixels);
            }
        }

        // TODO: This is the minimum clk cycles, make this accurate
        172 / 4 
    }
}

pub struct OamIter<'a> {
    oam_iter: std::slice::Iter<'a, OamEntry<'a>>,
    ly: u8,
    tile_height: u8,
    cnt: usize,
}

impl<'a> Iterator for OamIter<'a> {
    type Item = &'a OamEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // The PPU only generates the first 10
        if self.cnt == 10 {
            return None;
        }

        while let Some(oam_entry) = self.oam_iter.next() {
            let tile_y_pos = oam_entry.y_pos();

            if tile_y_pos == 0 || tile_y_pos >= 160 {
                // Tile is off screen
                continue;
            }

            // The Y coordinate of the OAM entry is
            // the screen coordinate + 16.  This allows
            // scrolling in from off screen.
            let adj_ly = self.ly.wrapping_add(16);

            if adj_ly > tile_y_pos && self.ly < tile_y_pos + self.tile_height {
                self.cnt += 1;
                return Some(oam_entry);
            }
        }

        None
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_oam_entry() {
        let y_pos = 0x34;
        let x_pos = 0x12;
        let tile_idx = 10;
        let attr = 0xF0;
        let bytes = [y_pos, x_pos, tile_idx, attr];
        let oam = OamEntry::from_bytes(&bytes);

        assert_eq!(oam.x_pos(), x_pos);
        assert_eq!(oam.y_pos(), y_pos);
        assert_eq!(oam.tile_idx(), tile_idx);
        let flags = oam.oam_flags();
        assert!(flags.priority);
        assert!(flags.x_flip);
        assert!(flags.y_flip);
        assert!(flags.dmg_palette);

        let attr = 0x00;
        let bytes = [y_pos, x_pos, tile_idx, attr];
        let oam = OamEntry::from_bytes(&bytes);
        let flags = oam.oam_flags();
        assert!(!flags.priority);
        assert!(!flags.x_flip);
        assert!(!flags.y_flip);
        assert!(!flags.dmg_palette);
    }


    #[test]
    fn oam_iterator_basic() {
        let mut mem = [0; 0xA0];


        // For 3 OAM entries, set Y to 0xAB
        mem[40] = 10;
        mem[41] = 0x00;
        mem[42] = 0x00;

        mem[44] = 10;
        mem[45] = 0x10;
        mem[46] = 0x10;

        mem[48] = 10;
        mem[49] = 0x20;
        mem[50] = 0x20;

        let oam_map = OamMap::from_mem(&mem);
        let oam_iter = oam_map.iter(0, false);

        let oams : Vec<_> = oam_iter.collect();
        assert_eq!(oams.len(), 3);
        assert_eq!(oams[0].y_pos(), 10);
        assert_eq!(oams[1].y_pos(), 10);
        assert_eq!(oams[2].y_pos(), 10);

        assert_eq!(oams[0].x_pos(), 0x00);
        assert_eq!(oams[1].x_pos(), 0x10);
        assert_eq!(oams[2].x_pos(), 0x20);

        assert_eq!(oams[0].tile_idx(), 0x00);
        assert_eq!(oams[1].tile_idx(), 0x10);
        assert_eq!(oams[2].tile_idx(), 0x20);



    }

    fn get_test_tiles<'a>() -> [Tile<'static>; 4] {
        static BYTES : [u8; 64] = [ 0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00,
        0xFF,0x00,0xFF,0x00,0xFF,0x00,0xFF,0x00,0xFF,0x00,0xFF,0x00,0xFF,0x00,0xFF,0x00,
        0x00,0xFF,0x00,0xFF,0x00,0xFF,0x00,0xFF,0x00,0xFF,0x00,0xFF,0x00,0xFF,0x00,0xFF,
        0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF ];
        assert_eq!(BYTES.len(), 64);

        let tiles = [
            Tile::from_bytes(&BYTES[0..16]),
            Tile::from_bytes(&BYTES[16..32]),
            Tile::from_bytes(&BYTES[32..48]),
            Tile::from_bytes(&BYTES[48..64]),
        ];

        tiles
    }


    #[test]
    fn oam_render_test() {

        let mut mem = [0; 0xA0];

        // 4 OAM entries that overlap
        // starting at 0
        mem[0] = 10;
        mem[1] = 8;
        mem[2] = 0;

        mem[4] = 10;
        mem[5] = 10;
        mem[6] = 1;

        mem[8] = 10;
        mem[9] = 12;
        mem[10] = 2;

        mem[12] = 10;
        mem[13] = 14;
        mem[14] = 3;

        let oam_map = OamMap::from_mem(&mem);
        let oam_iter = oam_map.iter(0, false);

        let oams : Vec<_> = oam_iter.collect();
        assert_eq!(oams.len(), 4);

        let mut pixels = [0; 256];
        let _ = oam_map.render_line(&mut pixels, &get_test_tiles(), 0, false);

        assert_eq!(pixels[0..14], [0, 0, 1, 1, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3]);
        for val in &pixels[14..] {
            assert_eq!(*val, 0);
        }
    }

}
