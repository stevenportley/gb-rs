
const TILE_LEN: usize = 8;

// 2 bytes produces 2 lines, 8 lines
pub const NTILES: usize = 384;

pub const VERT_TILES: usize = 18;
pub const HORIZ_TILES: usize = 20;

#[derive(Debug, Clone, Copy)]
pub struct Tile {
    pub pixels: [[u8; TILE_LEN]; TILE_LEN],
}

pub struct TileMap {
    pub tiles: [Tile; NTILES],
}

pub struct LineIter<'a> {
    tilemap: &'a TileMap,
    horiz_idx: usize,
    line_index: usize,
}

impl<'a> LineIter<'a> {
    pub fn from_tilemap(tilemap: &'a TileMap, line_index: usize) -> Self {
        LineIter {
            tilemap,
            horiz_idx: 0,
            line_index,
        }
    }
}

impl Iterator for LineIter<'_> { 
    type Item = u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.horiz_idx >= HORIZ_TILES * 8 {
            return None
        }

        //Tile index from the start of this line
        let line_start_tile = (self.line_index / 8) * HORIZ_TILES;

        //The tile offset into this line
        let line_tile_offset = self.horiz_idx / 8;

        let tile_idx =  line_start_tile + line_tile_offset;
        let line_in_tile = self.line_index % 8;
        let pixel_in_line = self.horiz_idx % 8;

        self.horiz_idx += 1;
        let this_tile = self.tilemap.tiles[tile_idx];
        let this_line = this_tile.pixels[line_in_tile];
        Some(this_line[pixel_in_line])
    }
}

impl Tile {
    pub fn from_bytes(raw: &[u8]) -> Self {
        let make_line = |b1: u8, b2: u8| -> [u8; 8] {
            let mut line = [0; 8];

            for i in 0..7 {
                // B1 is 64, B2 is 128
                line[i] = line[i] + if ((b1 << i) & 0x80) == 0x80 { 1 } else { 0 };
                line[i] = line[i] + if ((b2 << i) & 0x80) == 0x80 { 2 } else { 0 };
            }
            line
        };

        let mut this_tile = Tile {
            pixels: [[0; 8]; 8],
        };

        let mut b_iter = raw.iter().enumerate();

        while let Some((idx, b1)) = b_iter.next() {
            let line_index = idx / 2;
            let (_, b2) = b_iter.next().unwrap();

            this_tile.pixels[line_index].copy_from_slice(&make_line(*b1, *b2));
        }

        this_tile
    }
}

fn tile_to_pixel_idx(x : usize, y : usize) -> (usize, usize) {
    (x * 8, y * 8)
}
