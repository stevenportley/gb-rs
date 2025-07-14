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
