
pub struct Tile<'a> {
    data: &'a [u8],
}

impl<'a> Tile<'a> {
    pub fn from_bytes(data: &'a [u8]) -> Self {
        assert_eq!(data.len(), 16);
        Self { data }
    }

    pub fn pixel_buf(&self, line_idx: u8) -> [u8; 8] {
        assert!(line_idx < 8);

        let make_line = |b1: u8, b2: u8| -> [u8; 8] {
            let mut line = [0; 8];

            for i in 0..7 {
                // B1 is 64, B2 is 128
                line[i] = line[i] + if ((b1 << i) & 0x80) == 0x80 { 1 } else { 0 };
                line[i] = line[i] + if ((b2 << i) & 0x80) == 0x80 { 2 } else { 0 };
            }
            line
        };

        make_line(self.data[(line_idx * 2) as usize], self.data[((line_idx * 2) + 1) as usize])
    }
}

pub struct TileRenderer<'a> {
    tiles: &'a [Tile<'a>],
    image_width: usize,
    tile_cnt : usize,
    line_idx : usize,
}

impl<'a> TileRenderer<'a> {

    pub fn from_tiles(tiles: &'a [Tile<'_>], image_width: usize) -> Self {
        assert_eq!(image_width % 8, 0);
        Self {
            tiles,
            image_width,
            tile_cnt: 0,
            line_idx: 0,
        }
    }

}

impl<'a> Iterator for TileRenderer<'a> { 
    type Item = [u8; 8];

    fn next(&mut self) -> Option<Self::Item> {

        // The number of tiles in a horizontal line
        let num_tiles_in_line = self.image_width / 8;

        // The tile offset corresponding to the begining of this line
        let y_tile_offset = (self.line_idx / 8) * num_tiles_in_line;

        // The horizonal tile offset 
        let x_tile_offset = self.tile_cnt;

        let this_tile = y_tile_offset + x_tile_offset;

        if this_tile == self.tiles.len() {
            return None;
        }

        // vertical offset inside the tile
        // e.g. if we are drawing line 10, this should be 2
        // since we are drawing the third line inside the tile
        let vert_line_tile_offset : u8 = (self.line_idx % 8).try_into().unwrap();

        let pixel_indices = self.tiles[this_tile].pixel_buf(vert_line_tile_offset);

        if self.tile_cnt == num_tiles_in_line - 1 {
            self.line_idx += 1;
            self.tile_cnt = 0;
        } else {
            self.tile_cnt += 1
        }

        Some(pixel_indices)
    }

}



/*
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
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_test() {
        let raw = [
            0x7C, 0x7C, 0x00, 0xC6, 0xC6, 0x00, 0x00, 0xFE, 0xC6, 0xC6, 0x00, 0xC6, 0xC6, 0x00,
            0x00, 0x00,
        ];

        let tile = Tile::from_bytes(&raw);
        assert_eq!(tile.pixel_buf(0), [0, 3, 3, 3, 3, 3, 0, 0]);
        assert_eq!(tile.pixel_buf(1), [2, 2, 0, 0, 0, 2, 2, 0]);
        assert_eq!(tile.pixel_buf(2), [1, 1, 0, 0, 0, 1, 1, 0]);
        assert_eq!(tile.pixel_buf(3), [2, 2, 2, 2, 2, 2, 2, 0]);
        assert_eq!(tile.pixel_buf(4), [3, 3, 0, 0, 0, 3, 3, 0]);
        assert_eq!(tile.pixel_buf(5), [2, 2, 0, 0, 0, 2, 2, 0]);
        assert_eq!(tile.pixel_buf(6), [1, 1, 0, 0, 0, 1, 1, 0]);
        assert_eq!(tile.pixel_buf(7), [0, 0, 0, 0, 0, 0, 0, 0]);
    }
    
}
