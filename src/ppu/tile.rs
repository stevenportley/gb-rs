use bitfield_struct::bitfield;
use core::iter::IntoIterator;
use heapless::Vec;
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

    #[inline(always)]
    pub fn render<'a>(&self, dest: impl IntoIterator<Item = &'a mut u8>, palette: Palette) {
        let d_iter = dest.into_iter().take(8);
        let mut idx = 8;

        let b1 = self.data[0];
        let b2 = self.data[1];

        for d in d_iter {
            idx -= 1;
            // The corresponding bit in each byte that make
            // up the 2 index
            let _b2 = b2.checked_shr(idx).unwrap_or(0) & 0x1;
            let _b1 = b1.checked_shr(idx).unwrap_or(0) & 0x1;
            let color_id = (2 * _b2) + _b1;
            *d = Self::apply_palette(color_id, palette);
        }
    }
}

#[derive(FromBytes, Immutable, KnownLayout)]
pub struct Tile {
    pub lines: [Line; 8],
}

impl Tile {
    pub fn render_with_palette(&self, palette: Palette) -> [[u8; 8]; 8] {
        let mut tile = [[0; 8]; 8];

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

#[bitfield(u8)]
#[derive(FromBytes, Immutable, KnownLayout)]
pub struct OamFlags {
    #[bits(3)]
    _gcb_palette: u8,
    #[bits(1)]
    pub bank: bool,
    #[bits(1)]
    pub dmg_palette: bool,
    #[bits(1)]
    pub x_flip: bool,
    #[bits(1)]
    pub y_flip: bool,
    #[bits(1)]
    pub priority: bool,
}

#[derive(FromBytes, Immutable, KnownLayout, Clone, Copy)]
pub struct OamEntry {
    pub y: u8,
    pub x: u8,
    pub tile_idx: u8,
    pub flags: OamFlags,
}

impl OamEntry {
    pub fn render<'a, I: IntoIterator<Item = &'a mut u8>>(
        &self,
        vram: &VramBank,
        mut line_idx: u8,
        large_tiles: bool,
        palette: Palette,
        dest: I,
    ) where
        <I as IntoIterator>::IntoIter: DoubleEndedIterator,
    {
        if self.flags.y_flip() {
            line_idx = if large_tiles {
                15 - line_idx
            } else {
                7 - line_idx
            };
        }

        let mut tile_idx = self.tile_idx;
        if large_tiles {
            if line_idx >= 8 {
                tile_idx = tile_idx | 0x01;
                line_idx -= 8;
            } else {
                tile_idx = tile_idx & 0xFE;
            }
        }

        let tile: &Tile = &vram.tiles[tile_idx as usize];
        if self.flags.x_flip() {
            // TODO: This is wrong
            tile.lines[line_idx as usize].render(dest.into_iter().rev(), palette);
        } else {
            tile.lines[line_idx as usize].render(dest.into_iter(), palette);
        };
    }
}

#[derive(FromBytes, Immutable, KnownLayout)]
pub struct Oam {
    pub oam_entries: [OamEntry; 40],
}

impl Oam {
    pub fn get_oams_line(&self, line: u8, large_tiles: bool) -> Vec<OamEntry, 10> {
        // The PPU only generates the first 10
        let mut oams: Vec<OamEntry, 10> = Vec::new();

        let tile_height = if large_tiles { 16 } else { 8 };

        for oam_entry in &self.oam_entries {
            if oams.is_full() {
                break;
            }

            let tile_y_pos = oam_entry.y;

            if tile_y_pos == 0 || tile_y_pos >= 160 {
                // Tile is off screen
                continue;
            }

            // The Y coordinate of the OAM entry is
            // the screen coordinate + 16.  This allows
            // scrolling in from off screen.
            let adj_ly = line + 16;

            if adj_ly >= tile_y_pos && adj_ly < tile_y_pos + tile_height {
                // This will maintain a reverse-sorted list of OAM entries
                // by their X position. `<` is used rather than `<=` because
                // entries earlier in RAM are higher priority if X is the same.
                let idx = oams.partition_point(|&o| oam_entry.x < o.x);

                let _ = oams.insert(idx, *oam_entry);
            }
        }

        oams
    }
}
