#[derive(Clone, Copy)]
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

            line[0] = ((b2 >> 6) & 0x2) + ((b1 >> 7) & 1);
            line[1] = ((b2 >> 5) & 0x2) + ((b1 >> 6) & 1);
            line[2] = ((b2 >> 4) & 0x2) + ((b1 >> 5) & 1);
            line[3] = ((b2 >> 3) & 0x2) + ((b1 >> 4) & 1);
            line[4] = ((b2 >> 2) & 0x2) + ((b1 >> 3) & 1);
            line[5] = ((b2 >> 1) & 0x2) + ((b1 >> 2) & 1);
            line[6] = ((b2 >> 0) & 0x2) + ((b1 >> 1) & 1);
            line[7] = ((b2 << 1) & 0x2) + ((b1 >> 0) & 1);

            line
        };

        make_line(
            self.data[(line_idx * 2) as usize],
            self.data[((line_idx * 2) + 1) as usize],
        )
    }

    pub fn render(&self) -> [[u8; 8]; 8] {
        let tile: [[u8; 8]; 8] = core::array::from_fn(|index| self.pixel_buf(index as u8));

        tile
    }
}

/*

pub struct TileRenderer<'a> {
    tiles: &'a [Tile<'a>],
    image_width: usize,
    tile_cnt: usize,
    line_idx: usize,
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
        let vert_line_tile_offset: u8 = (self.line_idx % 8).try_into().unwrap();

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

pub fn get_background(tiles: &[Tile]) -> [[u8; 256]; 256] {
    assert_eq!(tiles.len(), 32 * 32);

    let mut pixels = [[0; 256]; 256];
    let mut tile_iter = TileRenderer::from_tiles(tiles, 32 * 8);
    let mut pixel_iter = pixels.iter_mut();

    while let Some(this_line) = pixel_iter.next() {
        let mut line_iter = this_line.chunks_exact_mut(8);
        while let Some(this_tile) = line_iter.next() {
            let tile_pixels = tile_iter.next().expect("Background size mismatch");
            this_tile.copy_from_slice(&tile_pixels);
        }
    }

    pixels
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
