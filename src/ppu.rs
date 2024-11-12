use crate::interrupts::IntSource;
use crate::tile::{get_background, Tile, TileRenderer};
use crate::oam::OamMap;

// The number of tiles in all of VRAM
pub const NTILES: usize = 384;

// The whole background
pub const NUM_BACKGROUND_TILES: usize = 32 * 32;

const VRAM_LEN: usize = 0x2000;
const OAM_LEN: usize = 0xA0;

#[derive(Clone, Copy)]
enum PpuMode {
    HBLANK = 0,
    VBLANK = 1,
    OAMSCAN = 2,
    DRAW = 3,
}

pub struct PPU {
    pub vram: [u8; VRAM_LEN],
    oam: [u8; OAM_LEN],
    lcdc: u8,
    stat: u8,
    scy: u8,
    scx: u8,
    ly: u8,
    lyc: u8,
    bgp: u8,
    obp0: u8,
    obp1: u8,
    wy: u8,
    wx: u8,
    curr_x: u8,
    mode: PpuMode,
    frame: [[u8; 160]; 144]
}

impl PPU {
    pub fn new() -> Self {
        Self {
            vram: [0; VRAM_LEN],
            oam: [0; OAM_LEN],
            //TODO: Are all of these actually 0 after POR?
            lcdc: 0,
            stat: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            bgp: 0,
            obp0: 0,
            obp1: 0,
            wy: 0,
            wx: 0,
            curr_x: 0,
            mode: PpuMode::OAMSCAN,
            frame: [[0; 160]; 144],
        }
    }

    pub fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x8000..=0x9FFF => {
                self.vram[addr as usize - 0x8000] = val;
            }
            0xFE00..=0xFE9f => {
                self.oam[addr as usize - 0xFE00] = val;
            }
            0xFF40 => {
                self.lcdc = val;
            }
            0xFF41 => {
                self.stat = val;
            }
            0xFF42 => {
                self.scy = val;
            }
            0xFF43 => {
                self.scx = val;
            }
            0xFF44 => {
                println!("Trying to write to LY, a read-only register");
            }
            0xFF45 => {
                self.lyc = val;
            }
            0xFF46 => {
                unimplemented!("DMA not implemented in PPU!")
            }
            0xFF47 => {
                self.bgp = val;
            }
            0xFF48 => {
                self.obp0 = val;
            }
            0xFF49 => {
                self.obp1 = val;
            }
            0xFF4A => {
                self.wy = val;
            }
            0xFF4B => {
                self.wx = val;
            }
            _ => {
                unreachable!("Invalid write to PPU? addr:{:?}, val:{:?}", addr, val);
            }
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0x9FFF => {
                return self.vram[addr as usize - 0x8000];
            }
            0xFE00..=0xFE9f => {
                return self.oam[addr as usize - 0xFE00];
            }
            0xFF40 => {
                return self.lcdc;
            }
            0xFF41 => {
                return self.get_stat();
            }
            0xFF42 => {
                return self.scy;
            }
            0xFF43 => {
                return self.scx;
            }
            0xFF44 => {
                return self.ly;
            }
            0xFF45 => {
                return self.lyc;
            }
            0xFF46 => {
                unimplemented!("Reading from DMA register!?!")
            }
            0xFF47 => {
                return self.bgp;
            }
            0xFF48 => {
                return self.obp0;
            }
            0xFF49 => {
                return self.obp1;
            }
            0xFF4A => {
                return self.wy;
            }
            0xFF4B => {
                return self.wx;
            }
            _ => {
                unreachable!("Invalid read from PPU? addr:{:?}", addr);
            }
        }
    }
    fn bkgr_start_addr(&self) -> usize {
        if self.lcdc & 0x8 == 0 {
            return 0x9800;
        } else {
            return 0x9C00;
        }
    }

    pub fn palette_to_rgba(ind: u8) -> [u8; 4] {
        let val = 255 - (85 * ind);
        return [val, val, val, 0xFF];
    }

    pub fn dump_vram(&self) -> [Tile; NTILES] {
        let tiles: [Tile; NTILES] = core::array::from_fn(|index| self.from_tile_index(index));

        return tiles;
    }

    pub fn from_tile_index(&self, tile_index: usize) -> Tile {
        let index = tile_index * 16;
        return Tile::from_bytes(&self.vram[index..index + 16]);
    }

    pub fn get_background(&self) -> [Tile; NUM_BACKGROUND_TILES] {
        let tiles: [Tile; NUM_BACKGROUND_TILES] = core::array::from_fn(|index| {
            let tilemap_index = index + self.bkgr_start_addr() - 0x8000;
            let tile_index = self.vram[tilemap_index];
            self.from_tile_index(tile_index as usize)
        });

        tiles
    }

    pub fn get_frame2(&self) -> [u8; (8*32) * (4*8*32)] {

        let mut pixels = [0; (8*32)*(4*8*32)];
        let bkgd_tiles = self.get_background();

        let mut bkgnd = crate::tile::get_background(&bkgd_tiles);

        let oam_map = OamMap::from_mem(&self.oam);
        for ly in 0..bkgnd.len() {
            //oam_map.render_line(&mut bkgnd[ly][0..160], &bkgd_tiles, ly as u8, false);
        }

        let mut bkgnd_iter = bkgnd.into_iter().flatten();

        for (_, one_pixel) in pixels.chunks_exact_mut(4).enumerate() {
            if let Some(new_pixel) = bkgnd_iter.next() {
                one_pixel.copy_from_slice(&Self::palette_to_rgba(new_pixel));
            }
        }

        pixels
    }

    //TODO: This function isn't going to work right,
    //      need to replace with an actual line renderer
    pub fn get_frame(&self) -> [u8; (8*32) * (4*8*32)] {
        let bck_gnd = self.get_background();
        let mut tile_renderer = TileRenderer::from_tiles(&bck_gnd, 32 * 8);

        let mut pixels = [0; (8*32)*(4*8*32)];

        for (_, eight_pixels) in pixels.chunks_exact_mut(4 * 8).enumerate() {
            if let Some(new_pixels) = tile_renderer.next() {
                for i in 0..8 {
                    eight_pixels[(4 * i)..((4 * i) + 4)]
                        .copy_from_slice(&Self::palette_to_rgba(new_pixels[i]));
                }
            }
        }

        pixels
    }

    // Get necessary information from PPU registers
    //  - SCX, SCY, etc.
    // Find first 10 suitable OAM entries (check Y coordinate)
    // Render (flipping, priority, etc) along with background / window

    /*
    pub fn get_line(&mut self) {


    }
    */

    pub fn run_one(&mut self) -> (u8, Option<IntSource>) {
        match self.mode {
            PpuMode::OAMSCAN => {
                self.mode = PpuMode::DRAW;
                // No interrupts can trigger 
                // when moving to DRAW
                return (20, None)
            }
            PpuMode::DRAW => {
                self.mode = PpuMode::HBLANK;

                // Update a line
                //TODO: Incorproate SCX
                //let adjusted_ly = self.ly.wrapping_add(self.scy);
                

                // Check for HBLANK interrupt
                // We are not considering 
                // LCD mode 1 (VLBLANK) interrupt
                // because is already has it's own
                // interupt? Why would nintendo 
                // do this?
                if (self.stat & 0x8) != 0 {
                    //TODO: Update this with actual
                    //      LCD timing (including delay)
                    //      rather than just using the minimum
                    return (43, Some(IntSource::LCD))
                } else {
                    return (43, None)
                }
            }

            PpuMode::HBLANK => {

                if self.ly == 143 {
                    self.ly += 1;
                    self.mode = PpuMode::VBLANK;
                    return (51, Some(IntSource::VBLANK));
                } else {
                    self.ly += 1;
                    self.mode = PpuMode::OAMSCAN;

                    // Check for LYC int
                    if (self.stat & 0x40) != 0 {
                        if self.ly == self.lyc {
                            return (51, Some(IntSource::LCD));
                        }
                    }

                    // Check for OAM scan interrupt
                    if (self.stat & 0x20) != 0 {
                        return (51, Some(IntSource::LCD));
                    } else {
                        return (51, None);
                    }
                }
            }

            PpuMode::VBLANK => {
                if self.ly == 153 {
                    // Go back OAM Scan and restart!
                    self.mode = PpuMode::OAMSCAN;
                    self.ly = 0;
                    // Check for OAM scan interrupt
                    if (self.stat & 0x20) != 0 {
                        return (114, Some(IntSource::LCD));
                    } else {
                        return (114, None);
                    }
                } else {
                    self.ly += 1;
                    return (114, None);
                }
            }

        }

    }

    pub fn tick(&mut self) -> Option<IntSource> {
        if self.curr_x == 113 {
            self.curr_x = 0;
            if self.ly == 153 {
                self.ly = 0;
                return None;
            } else {
                self.ly += 1;
            }
        } else {
            self.curr_x += 1;
        }

        if self.ly >= 144 {
            self.mode = PpuMode::VBLANK;
            return Some(IntSource::VBLANK);
        } else if self.curr_x < 20 {
            self.mode = PpuMode::OAMSCAN;
        } else if self.curr_x >= 94 {
            self.mode = PpuMode::HBLANK;
        } else {
            self.mode = PpuMode::DRAW;
        }

        return None;
    }

    fn get_stat(&self) -> u8 {
        let base = self.stat & !0x7;
        return base | self.mode as u8 | 
            if self.ly == self.lyc { 0x6 } else { 0 };
    }
}
