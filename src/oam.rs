use crate::tile::Tile;
use heapless::Vec;

pub struct OamEntry<'a> {
    data: &'a [u8],
}

pub struct OamFlags {
    pub low_priority: bool,
    pub y_flip: bool,
    pub x_flip: bool,
    pub dmg_palette: bool,
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

        if flags.y_flip {
            line_idx = if large_tiles {
                15 - line_idx
            } else {
                7 - line_idx
            };
        }

        if large_tiles {
            if line_idx >= 8 {
                tile_idx = tile_idx | 0x01;
                line_idx -= 8;
            } else {
                tile_idx = tile_idx & 0xFE;
            }
        }

        let tile = &tiles[tile_idx as usize];

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
        let mut oams: Vec<&OamEntry, 10> = Vec::new();

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
                // This will maintain a reverse-sorted list of OAM entries
                // by their X position. `<` is used rather than `<=` because
                // entries earlier in RAM are higher priority if X is the same.
                let idx = oams.partition_point(|&o| oam_entry.x_pos() < o.x_pos());

                let _ = oams.insert(idx, oam_entry);
            }
        }

        oams
    }

    pub fn get_oams_screen(&self) -> Vec<&OamEntry, 40> {
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

    fn get_weird_tile() -> Tile<'static> {
        static BYTES: [u8; 16] = [
            0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0x0F, 0xF0, 0x0F, 0xF0, 0x0F, 0xF0, 0x0F,
            0xF0, 0x0F,
        ];

        Tile::from_bytes(&BYTES)
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
