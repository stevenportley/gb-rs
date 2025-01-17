use crate::interrupts::IntSource;
use crate::oam::OamMap;
use crate::tile::Tile;

// The number of tiles in all of VRAM
pub const NTILES: usize = 384;

// The whole background
pub const TILE_MAP_LEN: usize = 32 * 32;

pub const BKG_WIDTH: usize = 256;
pub const SCREEN_WIDTH: usize = 160;
pub const SCREEN_HEIGHT: usize = 144;

const VRAM_LEN: usize = 0x2000;
const OAM_LEN: usize = 0xA0;

#[derive(Clone, Copy, Debug)]
pub enum PpuMode {
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
    mode: PpuMode,
    r_cyc: i32,
    pub screen: Frame,
}

#[derive(Debug)]
pub struct Lcdc {
    pub lcd_en: bool,
    pub window_tile_map: bool,
    pub window_en: bool,
    pub bg_wind_tile_data: bool,
    pub bg_tile_map: bool,
    pub large_sprite: bool,
    pub obj_en: bool,
    pub bg_wind_en: bool,
}

#[derive(Debug)]
pub struct PpuState {
    pub lcdc: Lcdc,
    pub scx: u8,
    pub scy: u8,
    pub ly: u8,
    pub mode: PpuMode,
    pub lyc: u8,
    pub stat: u8,
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
            mode: PpuMode::OAMSCAN,
            r_cyc: 20,
            screen: Frame::new(),
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
                //println!("Trying to write to LY, a read-only register");
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
                return 0xFF;
                //unimplemented!("Reading from DMA register!?!")
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

    fn render_tiles(tiles: &[Tile; TILE_MAP_LEN], line: u8) -> [u8; BKG_WIDTH] {
        // The number of tiles in a horizontal line
        const N_TILES_IN_LINE: usize = BKG_WIDTH / 8;

        // The tile offset corresponding to the begining of this line
        let y_tile_offset = (line as usize / 8) * N_TILES_IN_LINE;

        // vertical offset inside the tile
        // e.g. if we are drawing line 10, this should be 2
        // since we are drawing the third line inside the tile
        let vert_line_tile_offset: u8 = (line % 8).try_into().unwrap();

        let mut pixels = [0; BKG_WIDTH];
        let mut i = 0;

        for (_, eight_pixels) in pixels.chunks_exact_mut(8).enumerate() {
            eight_pixels
                .copy_from_slice(&tiles[y_tile_offset + i].pixel_buf(vert_line_tile_offset));
            i += 1;
        }

        pixels
    }

    fn render_bg_line(&self, ly: u8) -> [u8; BKG_WIDTH] {
        Self::render_tiles(&self.get_background_tiles(), ly)
    }

    fn render_window_line(&self, ly: u8) -> [u8; BKG_WIDTH] {
        //TODO: We can do optimizations for rendering BG w/ Window
        // 1. If tile maps are the same, we only need to arrange the tiles once
        // 2. Consider keeping around all of the tiles and only update them
        //    when VRAM is updated
        Self::render_tiles(&self.get_window_tiles(), ly)
    }

    pub fn render_bg(&self) -> [[u8; BKG_WIDTH]; BKG_WIDTH] {
        let bkg: [[u8; BKG_WIDTH]; BKG_WIDTH] =
            core::array::from_fn(|index| self.render_bg_line(index as u8));

        bkg
    }

    pub fn render_window(&self) -> [[u8; BKG_WIDTH]; BKG_WIDTH] {
        let bkg: [[u8; BKG_WIDTH]; BKG_WIDTH] =
            core::array::from_fn(|index| self.render_window_line(index as u8));

        bkg
    }

    fn obj_en(&self) -> bool {
        self.lcdc & 0x2 != 0
    }

    fn bkgr_map_start_addr(&self) -> u16 {
        if self.lcdc & 0x8 == 0 {
            0x9800
        } else {
            0x9C00
        }
    }

    fn window_map_start_addr(&self) -> u16 {
        if self.lcdc & 0x40 == 0 {
            0x9800
        } else {
            0x9C00
        }
    }

    pub fn palette_to_rgba(ind: u8) -> [u8; 4] {
        let val = 255 - (85 * ind);
        return [val, val, val, 0xFF];
    }

    fn get_tile_map(&self, start_addr: u16) -> [u8; TILE_MAP_LEN] {
        let start_addr = start_addr as usize;
        let tile_map: [u8; TILE_MAP_LEN] = core::array::from_fn(|index| {
            let tilemap_index = index + start_addr - 0x8000;
            self.vram[tilemap_index]
        });

        tile_map
    }

    fn get_background_tiles(&self) -> [Tile; TILE_MAP_LEN] {
        let tile_map = self.get_tile_map(self.bkgr_map_start_addr());

        let tiles: [Tile; TILE_MAP_LEN] =
            core::array::from_fn(|index| self.bkgr_tile(tile_map[index]));

        tiles
    }

    fn get_window_tiles(&self) -> [Tile; TILE_MAP_LEN] {
        let tile_map = self.get_tile_map(self.window_map_start_addr());

        let tiles: [Tile; TILE_MAP_LEN] =
            core::array::from_fn(|index| self.bkgr_tile(tile_map[index]));

        tiles
    }

    pub fn bkgr_tile(&self, tile_index: u8) -> Tile {
        if self.lcdc & 0x10 == 0 {
            if tile_index < 128 {
                let tile_data = &self.vram[0x1000..0x1800];
                let tile_index = tile_index as usize * 16;
                Tile::from_bytes(&tile_data[tile_index..tile_index + 16])
            } else {
                let tile_data = &self.vram[0x0800..0x1000];
                let tile_index = (tile_index - 128) as usize * 16;
                Tile::from_bytes(&tile_data[tile_index..tile_index + 16])
            }
        } else {
            //assert!(false);
            let tile_data = &self.vram[..0x1000];
            let tile_index = tile_index as usize * 16;
            Tile::from_bytes(&tile_data[tile_index..tile_index + 16])
        }
    }

    pub fn from_oam_tile_index(&self, tile_index: usize) -> Tile {
        let index = tile_index * 16;
        return Tile::from_bytes(&self.vram[index..index + 16]);
    }

    pub fn get_screen(&self) -> [u8; 4 * SCREEN_WIDTH * SCREEN_HEIGHT] {
        self.screen.to_rgba()
    }

    pub fn get_sprite_map(&self) -> OamMap {
        OamMap::from_mem(&self.oam)
    }

    pub fn get_sprite_tile(&self, tile_index: usize) -> Tile {
        let vram_index = tile_index * 16;
        Tile::from_bytes(&self.vram[vram_index..vram_index + 16])
    }

    pub fn run(&mut self, cycles: i32) -> Option<IntSource> {
        if cycles < self.r_cyc {
            self.r_cyc = self.r_cyc - cycles;
            return None;
        }

        let over_cycles = cycles - self.r_cyc;

        match self.mode {
            PpuMode::OAMSCAN => {
                // 43 is the minimum, real should be
                // based on PPU / OAM state
                self.mode = PpuMode::DRAW;
                self.r_cyc = 43 - over_cycles;
            }

            PpuMode::DRAW => {
                // Exiting DRAW state

                let bg_line = self.render_bg_line(self.ly.wrapping_add(self.scy));
                let ly = self.ly as usize;

                let scx = self.scx as usize;

                //TODO: Cleaner way to do this?
                if scx + 160 > 255 {
                    let first_cpy_len = bg_line[scx..].len() as usize;
                    self.screen.buf[ly][0..first_cpy_len].copy_from_slice(&bg_line[scx..]);
                    self.screen.buf[ly][first_cpy_len..]
                        .copy_from_slice(&bg_line[..160 - first_cpy_len]);
                } else {
                    self.screen.buf[ly].copy_from_slice(&bg_line[scx..scx + 160]);
                }

                // Window is enabled
                if self.lcdc & 0x20 != 0 {
                    if self.ly >= self.wy {
                        let wx = self.wx as usize;
                        let window_line = self.render_window_line(self.ly - self.wy);
                        let screen_line = &mut self.screen.buf[ly];

                        if wx < 8 {
                            let window_offset = 7 - wx;
                            screen_line
                                .copy_from_slice(&window_line[window_offset..window_offset + 160]);
                        } else if wx > 166 {
                            // Window not visible
                        } else {
                            let screen_offset = wx - 7;
                            let window_len = 160 - screen_offset;
                            screen_line[screen_offset..]
                                .copy_from_slice(&window_line[..window_len]);
                        }
                    }
                }

                if self.obj_en() {
                    let oam_map = OamMap::from_mem(&self.oam);

                    let sprite_tiles: [Tile; 256] = core::array::from_fn(|tile_index| {
                        let vram_index = tile_index * 16;
                        Tile::from_bytes(&self.vram[vram_index..vram_index + 16])
                    });

                    let large_sprites = self.large_sprites();
                    oam_map.render_line(
                        &mut self.screen.buf[ly],
                        &sprite_tiles,
                        self.ly,
                        large_sprites,
                    );
                }

                // TODO: Use actual timing, not just 51
                self.mode = PpuMode::HBLANK;
                self.r_cyc = 51 - over_cycles;

                // Check for HBlank interrupt
                if (self.stat & 0x8) != 0 {
                    return Some(IntSource::LCD);
                }
            }

            PpuMode::HBLANK => {
                self.ly += 1;

                // Are we entering VBLANK?
                if self.ly == 143 {
                    self.mode = PpuMode::VBLANK;
                    self.r_cyc = 114 - over_cycles;
                    // Check for LYC int
                    if (self.stat & 0x40) != 0 {
                        if self.ly == self.lyc {
                            return Some(IntSource::LCD);
                        }
                    }
                    return Some(IntSource::VBLANK);
                } else {
                    self.mode = PpuMode::OAMSCAN;
                    self.r_cyc = 20 - over_cycles;

                    // Check for LYC int
                    if (self.stat & 0x40) != 0 {
                        if self.ly == self.lyc {
                            return Some(IntSource::LCD);
                        }
                    }

                    // Check for OAM scan interrupt
                    if (self.stat & 0x20) != 0 {
                        return Some(IntSource::LCD);
                    }
                }
            }

            PpuMode::VBLANK => {
                if self.ly == 153 {
                    // Go back OAM Scan and restart!
                    self.mode = PpuMode::OAMSCAN;
                    self.r_cyc = 20 - over_cycles;
                    self.ly = 0;

                    // Check for OAM scan interrupt
                    if (self.stat & 0x20) != 0 {
                        return Some(IntSource::LCD);
                    }
                } else {
                    self.ly += 1;
                    self.r_cyc = 114 - over_cycles;

                    // Check for LYC int
                    if (self.stat & 0x40) != 0 {
                        if self.ly == self.lyc {
                            return Some(IntSource::LCD);
                        }
                    }
                }
            }
        }

        None
    }

    fn get_stat(&self) -> u8 {
        let base = self.stat & !0x7;
        return base | self.mode as u8 | if self.ly == self.lyc { 0x6 } else { 0 };
    }

    fn get_lcdc_state(&self) -> Lcdc {
        Lcdc {
            lcd_en: self.lcdc & 0x80 != 0,
            window_tile_map: self.lcdc & 0x40 != 0,
            window_en: self.lcdc & 0x20 != 0,
            bg_wind_tile_data: self.lcdc & 0x10 != 0,
            bg_tile_map: self.lcdc & 0x08 != 0,
            large_sprite: self.large_sprites(),
            obj_en: self.lcdc & 0x02 != 0,
            bg_wind_en: self.lcdc & 0x01 != 0,
        }
    }

    pub fn get_ppu_state(&self) -> PpuState {
        PpuState {
            lcdc: self.get_lcdc_state(),
            scx: self.scx,
            scy: self.scy,
            ly: self.ly,
            lyc: self.lyc,
            mode: self.mode,
            stat: self.stat,
        }
    }

    fn large_sprites(&self) -> bool {
        self.lcdc & 0x04 != 0
    }
}

pub struct Frame {
    pub buf: [[u8; SCREEN_WIDTH]; SCREEN_HEIGHT],
}

impl Frame {
    pub fn new() -> Self {
        Frame {
            buf: [[0; SCREEN_WIDTH]; SCREEN_HEIGHT],
        }
    }

    pub fn to_rgba(&self) -> [u8; 4 * SCREEN_WIDTH * SCREEN_HEIGHT] {
        let mut pixels = [0; 4 * SCREEN_WIDTH * SCREEN_HEIGHT];

        let mut frame_iter = self.buf.into_iter().flatten();

        for (_, one_pixel) in pixels.chunks_exact_mut(4).enumerate() {
            if let Some(new_pixel) = frame_iter.next() {
                one_pixel.copy_from_slice(&PPU::palette_to_rgba(new_pixel));
            }
        }

        pixels
    }
}
