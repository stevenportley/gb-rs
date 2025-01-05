use crate::tile::Tile;
use heapless::Vec;

pub struct OamEntry<'a> {
    data: &'a [u8],
}

pub struct OamFlags {
    low_priority: bool,
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
            low_priority: (flags & 0x80 != 0),
            y_flip: (flags & 0x40 != 0),
            x_flip: (flags & 0x20 != 0),
            dmg_palette: (flags & 0x10 != 0),
            //bank: (flags & 0x08 != 0),
        }
    }

    pub fn get_pixels(&self, tiles: &[Tile], mut line_idx: u8, large_tiles: bool) -> [u8; 8] {
        let flags = self.oam_flags();
        let mut tile_idx = self.tile_idx();

        if large_tiles {
            if line_idx >= 8 {
                tile_idx = tile_idx & 0xFE;
                line_idx -= 8;
            } else {
                tile_idx = tile_idx | 0x01;
            }
        }

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

    pub fn get_oams_line(&self, ly: u8, large_tiles: bool) -> Vec<&OamEntry, 10> {
        // The PPU only generates the first 10

        let mut oams = Vec::new();

        let tile_height = if large_tiles { 16 } else { 8 };

        for oam_entry in &self.oam_entries {
            if oams.is_full() {
                break;
            }

            let tile_y_pos = oam_entry.y_pos();

            if tile_y_pos == 0 || tile_y_pos >= 160 {
                // Tile is off screen
                continue;
            }

            // The Y coordinate of the OAM entry is
            // the screen coordinate + 16.  This allows
            // scrolling in from off screen.
            let adj_ly = ly + 16;

            if adj_ly >= tile_y_pos && adj_ly < tile_y_pos + tile_height {
                let _ = oams.push(oam_entry);
            }
        }

        oams.sort_unstable_by(|l, r| r.x_pos().cmp(&l.x_pos()));
        oams
    }

    pub fn get_oams_screen(&self) -> Vec<&OamEntry, 20> {
        let mut oams = Vec::new();

        for oam_entry in &self.oam_entries {
            if oams.is_full() {
                return oams;
            }

            let tile_y_pos = oam_entry.y_pos();
            if tile_y_pos == 0 || tile_y_pos >= 160 {
                // Tile is off screen
                continue;
            }

            let _ = oams.push(oam_entry);
        }

        oams
    }

    pub fn render_line(&self, pixels: &mut [u8], tiles: &[Tile], ly: u8, large_tiles: bool) -> u32 {
        assert_eq!(pixels.len(), 160);

        let oams = self.get_oams_line(ly, large_tiles);

        for oam in oams {
            let x = oam.x_pos() as usize;

            if x == 0 || x >= 168 {
                // Off the screen
                continue;
            }

            // Shift LY to sprite y_pos space,
            // it's offset by 16 to allow scrolling in
            let sprite_offset = (ly + 16) - oam.y_pos();

            let oam_pixels = oam.get_pixels(tiles, sprite_offset, large_tiles);

            let (dst, src) = {
                if x < 8 {
                    // Clipped at beginning of line
                    (&mut pixels[..x], &oam_pixels[8 - x..])
                    //pixels[..x].copy_from_slice(&oam_pixels[8 - x..]);
                } else if x > 160 {
                    // Clipped at end of line
                    let b = 168 - x;
                    (&mut pixels[x - 8..], &oam_pixels[..b])
                    //pixels[x - 8..].copy_from_slice(&oam_pixels[..b]);
                } else {
                    (&mut pixels[x - 8..x], &oam_pixels[..])
                    //pixels[x - 8..x].copy_from_slice(&oam_pixels);
                }
            };

            assert!(dst.len() == src.len());

            //TODO: Find a cleaner way to do this

            if oam.oam_flags().low_priority {
                for i in 0..dst.len() {
                    if dst[i] == 0 {
                        dst[i] = src[i];
                    }
                }
            } else {
                for i in 0..dst.len() {
                    if src[i] != 0 {
                        dst[i] = src[i];
                    }
                }
            }
        }

        // TODO: This is the minimum clk cycles, make this accurate
        172 / 4
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
        assert!(flags.low_priority);
        assert!(flags.x_flip);
        assert!(flags.y_flip);
        assert!(flags.dmg_palette);

        let attr = 0x00;
        let bytes = [y_pos, x_pos, tile_idx, attr];
        let oam = OamEntry::from_bytes(&bytes);
        let flags = oam.oam_flags();
        assert!(!flags.low_priority);
        assert!(!flags.x_flip);
        assert!(!flags.y_flip);
        assert!(!flags.dmg_palette);
    }

    fn get_test_tiles() -> [Tile<'static>; 4] {
        static BYTES: [u8; 64] = [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00,
            0xFF, 0x00, 0xFF, 0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF,
            0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];
        assert_eq!(BYTES.len(), 64);

        let tiles = [
            Tile::from_bytes(&BYTES[0..16]),
            Tile::from_bytes(&BYTES[16..32]),
            Tile::from_bytes(&BYTES[32..48]),
            Tile::from_bytes(&BYTES[48..64]),
        ];

        tiles
    }

    fn get_weird_tile() -> Tile<'static> {
        static BYTES: [u8; 16] = [
            0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0xF0, 0x0F, 0xF0, 0x0F, 0xF0, 0x0F,
            0xF0, 0x0F,
        ];

        Tile::from_bytes(&BYTES)
    }

    #[test]
    fn oam_overlap() {
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
        let oams = oam_map.get_oams_screen();

        assert_eq!(oams.len(), 4);

        let mut pixels = [0; 160];
        let _ = oam_map.render_line(&mut pixels, &get_test_tiles(), 0, false);

        // TODO: Make sure that this is actually what we want
        assert_eq!(pixels[0..14], [0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 3, 3]);
        for val in &pixels[14..] {
            assert_eq!(*val, 0);
        }
    }

    #[test]
    fn oam_blank() {
        let mem = [0; 0xA0];
        let oam_map = OamMap::from_mem(&mem);
        let oams = oam_map.get_oams_screen();

        assert_eq!(oams.len(), 0);

        let mut pixels = [0; 160];
        let _ = oam_map.render_line(&mut pixels, &get_test_tiles(), 0, false);
        assert_eq!(pixels, [0; 160]);
    }

    #[test]
    fn oam_single_sprite() {
        let mut mem = [0; 0xA0];
        mem[8] = 10;
        mem[9] = 0x50;
        mem[10] = 2;

        let oam_map = OamMap::from_mem(&mem);
        let oams = oam_map.get_oams_screen();

        assert_eq!(oams.len(), 1);

        let mut pixels = [0; 160];
        let _ = oam_map.render_line(&mut pixels, &get_test_tiles(), 0, false);

        let x_pos = (oams[0].x_pos() - 8) as usize;
        assert_eq!(pixels[x_pos..x_pos + 8], [2; 8]);
    }

    #[test]
    fn oam_left_clip() {
        let mut mem = [0; 0xA0];
        mem[8] = 10;
        mem[9] = 2;
        mem[10] = 1;

        let oam_map = OamMap::from_mem(&mem);
        let oams = oam_map.get_oams_screen();

        assert_eq!(oams.len(), 1);

        let mut pixels = [0; 160];
        let _ = oam_map.render_line(&mut pixels, &get_test_tiles(), 0, false);

        assert_eq!(pixels[0..2], [1, 1]);
        assert_eq!(pixels[2..], [0; 158]);
    }

    #[test]
    fn oam_right_clip() {
        let mut mem = [0; 0xA0];
        mem[8] = 10;
        mem[9] = 164;
        mem[10] = 1;

        let oam_map = OamMap::from_mem(&mem);
        let oams = oam_map.get_oams_screen();
        assert_eq!(oams.len(), 1);

        let mut pixels = [0; 160];
        let _ = oam_map.render_line(&mut pixels, &get_test_tiles(), 0, false);

        assert_eq!(pixels[0..156], [0; 156]);
        assert_eq!(pixels[156..], [1; 4]);
    }

    #[test]
    fn oam_y_flip() {
        let mut mem = [0; 0xA0];
        mem[8] = 10;
        mem[9] = 16;
        mem[10] = 0;
        mem[11] = 0x40; // just bit 6, y_flip

        let oam_map = OamMap::from_mem(&mem);
        let oams = oam_map.get_oams_screen();

        assert_eq!(oams.len(), 1);

        assert_eq!(
            oams[0].get_pixels(&[get_weird_tile()], 0, false),
            [1, 1, 1, 1, 2, 2, 2, 2]
        );
        assert_eq!(
            oams[0].get_pixels(&[get_weird_tile()], 7, false),
            [0, 0, 0, 0, 3, 3, 3, 3]
        );
    }

    #[test]
    fn oam_x_flip() {
        let mut mem = [0; 0xA0];
        mem[8] = 10;
        mem[9] = 16;
        mem[10] = 0;
        mem[11] = 0x20; // just bit 5, x_flip

        let oam_map = OamMap::from_mem(&mem);
        let oams = oam_map.get_oams_screen();
        assert_eq!(oams.len(), 1);

        assert_eq!(
            oams[0].get_pixels(&[get_weird_tile()], 0, false),
            [3, 3, 3, 3, 0, 0, 0, 0]
        );
        assert_eq!(
            oams[0].get_pixels(&[get_weird_tile()], 7, false),
            [2, 2, 2, 2, 1, 1, 1, 1]
        );
    }

    #[test]
    fn oam_vert_spacing() {
        let mut mem = [0; 0xA0];
        mem[0] = 16;
        mem[1] = 8;
        mem[2] = 0;
        mem[3] = 0x00;

        let oam_exists = |ly| {
            let oam_map = OamMap::from_mem(&mem);
            let oams = oam_map.get_oams_line(ly, false);
            oams.len() == 1
        };

        assert!(oam_exists(0));
        assert!(oam_exists(1));
        assert!(oam_exists(7));
        assert!(!oam_exists(8));
        assert!(!oam_exists(16));
        assert!(!oam_exists(20));
    }
}
