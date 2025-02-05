use gb_rs::{
    ppu::{BKG_WIDTH, PPU, SCREEN_HEIGHT, SCREEN_WIDTH},
    tile::Tile,
};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Color,
    widgets::{
        canvas::{Canvas, Painter, Shape},
        Block, Widget,
    },
};

pub struct GameFrame<'a>(pub &'a PPU);
pub struct Background<'a>(pub &'a PPU);
pub struct BkWindow<'a>(pub &'a PPU);
pub struct TileShape<'a>(pub Tile<'a>);

impl<'a> Shape for GameFrame<'a> {
    fn draw(&self, painter: &mut Painter<'_, '_>) {
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let color = to_color(self.0.screen.buf[SCREEN_HEIGHT - y - 1][x]);
                if let Some((x, y)) = painter.get_point(x as f64, y as f64) {
                    painter.paint(x, y, color);
                }
            }
        }
    }
}

impl<'a> Shape for Background<'a> {
    fn draw(&self, painter: &mut Painter<'_, '_>) {
        let bkgr = self.0.render_bg();
        for y in 0..BKG_WIDTH {
            for x in 0..BKG_WIDTH {
                let color = to_color(bkgr[BKG_WIDTH - y - 1][x]);
                if let Some((x, y)) = painter.get_point(x as f64, y as f64) {
                    painter.paint(x, y, color);
                }
            }
        }
    }
}

impl<'a> Shape for BkWindow<'a> {
    fn draw(&self, painter: &mut Painter<'_, '_>) {
        let bkgr = self.0.render_window();
        for y in 0..BKG_WIDTH {
            for x in 0..BKG_WIDTH {
                let color = to_color(bkgr[BKG_WIDTH - y - 1][x]);
                if let Some((x, y)) = painter.get_point(x as f64, y as f64) {
                    painter.paint(x, y, color);
                }
            }
        }
    }
}

impl Shape for TileShape<'_> {
    fn draw(&self, painter: &mut Painter<'_, '_>) {
        let oam_tile = self.0.render();
        for y in 0..8 {
            for x in 0..8 {
                if let Some((x2, y2)) = painter.get_point(x as f64, y as f64) {
                    painter.paint(x2, y2, to_color(oam_tile[7 - y][x]));
                }
            }
        }
    }
}

pub struct GameWidget<'a>(pub &'a PPU);
pub struct SpritesWidget<'a>(pub &'a PPU);
pub struct TilesetWidget<'a>(pub &'a [Tile<'a>], pub &'static str);

impl Widget for GameWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let outer_block = Block::bordered().title("Main Screen");
        let inner_area = outer_block.inner(area);

        let canvas = Canvas::default()
            .marker(ratatui::symbols::Marker::HalfBlock)
            .paint(|ctx| {
                let game_frame = GameFrame(&self.0);
                ctx.draw(&game_frame);
            })
            .x_bounds([0.0, SCREEN_WIDTH as f64])
            .y_bounds([0.0, SCREEN_HEIGHT as f64]);

        outer_block.render(area, buf);
        canvas.render(inner_area, buf);
    }
}

impl Widget for SpritesWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        const GRID_LEN: usize = 7;
        const TILE_LEN: u16 = 8;

        let sprite_map = self.0.get_sprite_map();
        let oams = sprite_map.get_oams_screen();

        let outer_block = Block::bordered().title(format!("Sprites ({})", oams.len()));
        let inner = outer_block.inner(area);

        let grid = {
            let mut grid = Vec::new();

            let vert = Layout::vertical(Constraint::from_lengths([TILE_LEN / 2; GRID_LEN]))
                .flex(ratatui::layout::Flex::SpaceAround);
            let slots: [Rect; GRID_LEN] = vert.areas(inner);

            let horiz = Layout::horizontal(Constraint::from_lengths([TILE_LEN; GRID_LEN]))
                .flex(ratatui::layout::Flex::SpaceAround);

            for slot in slots {
                let grid_slots = horiz.split(slot);
                for grid_slot in grid_slots.iter() {
                    grid.push(grid_slot.clone());
                }
            }

            grid
        };

        let mut oam_iter = oams.iter();

        for slot in grid {
            if let Some(data) = oam_iter.next() {
                let canvas = Canvas::default()
                    .marker(ratatui::symbols::Marker::HalfBlock)
                    .paint(|ctx| {
                        let tile = self.0.get_sprite_tile(data.tile_idx().into());
                        ctx.draw(&TileShape(tile));
                    })
                    .x_bounds([0.0, 8.0])
                    .y_bounds([0.0, 8.0]);

                canvas.render(slot, buf);
            }
        }

        outer_block.render(area, buf);
    }
}

/*
impl Widget for TilesetWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {

        let outer_block = Block::bordered().title(format!("{} Tiles", self.1));
        let inner = outer_block.inner(area);


        let vert = Layout::vertical(Constraint::from_lengths([TILE_LEN / 2; GRID_LEN]))
                .flex(ratatui::layout::Flex::SpaceAround);
            let slots: [Rect; GRID_LEN] = vert.areas(inner);

            let horiz = Layout::horizontal(Constraint::from_lengths([TILE_LEN; GRID_LEN]))
                .flex(ratatui::layout::Flex::SpaceAround);

    }
}
*/

fn to_color(color: u8) -> Color {
    match color {
        0 => Color::White,
        1 => Color::Gray,
        2 => Color::DarkGray,
        3 => Color::Black,
        _ => Color::Blue,
    }
}
