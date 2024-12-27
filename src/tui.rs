use gb_rs::{gb::GbRs, joypad::JoypadDirection, joypad::JoypadInput, tile::Tile};
use std::io;

use std::time::{Duration, Instant};

use ratatui::layout::{Constraint, Layout};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::event::{
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::execute;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Color,
    text::{Line, Text},
    widgets::{
        canvas::{Canvas, Painter, Shape},
        Block, Paragraph, Widget,
    },
    DefaultTerminal, Frame,
};

const SCREEN_WIDTH: usize = 160;
const SCREEN_HEIGHT: usize = 144;

pub struct App {
    counter: u32,
    halt: bool,
    exit: bool,
    gb: GbRs,
    frame_time: Duration,
    emu_time: Duration,
}

struct GameFrame<'a> {
    frame: &'a gb_rs::ppu::Frame,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            let before = Instant::now();
            if !self.halt {
                self.gb.run_frame();
            }
            self.emu_time = Instant::now() - before;
            terminal.draw(|frame| self.draw(frame))?;
            self.frame_time = Instant::now() - before;
            self.handle_events()?;
            self.counter += 1;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let canvas = Canvas::default()
            //.block(Block::bordered())
            .marker(ratatui::symbols::Marker::HalfBlock)
            .paint(|ctx| {
                let game_frame = GameFrame {
                    frame: &self.gb.cpu.bus.ppu.screen,
                };
                ctx.draw(&game_frame);
            })
            .x_bounds([0.0, SCREEN_WIDTH as f64])
            .y_bounds([0.0, SCREEN_HEIGHT as f64]);

        let horizontal =
            Layout::horizontal([Constraint::Length(SCREEN_WIDTH as u16), Constraint::Fill(1)]);
        let [left, right] = horizontal.areas(frame.area());

        let sub_vert = Layout::vertical(Constraint::from_percentages([20, 80]));
        let [top_right, bottom_right] = sub_vert.areas(right);


        let vertical = Layout::vertical([
            Constraint::Length(SCREEN_HEIGHT as u16 / 2),
            Constraint::Fill(1),
        ]);
        let [main, _] = vertical.areas(left);


        frame.render_widget(
            Block::bordered().title(format!(
                "GB RS: Area: {:?}, Frame: {}",
                left,
                &self.counter.to_string()
            )),
            right,
        );

        frame.render_widget(canvas, main);

        let joypad_state = self.gb.cpu.bus.joypad.get_state();
        frame.render_widget(Text::from(format!("{}", joypad_state)), right);

        let instr_trace = self.gb.cpu.get_next_instrs::<20>();

        frame.render_widget(
            Paragraph::new(vec![
                Line::from(format!("{:?}", instr_trace[0])),
                Line::from(format!("{:?}", instr_trace[1])),
                Line::from(format!("{:?}", instr_trace[2])),
                Line::from(format!("{:?}", instr_trace[3])),
                Line::from(format!("{:?}", instr_trace[4])),
                Line::from(format!("{:?}", instr_trace[5])),
                Line::from(format!("{:?}", instr_trace[6])),
                Line::from(format!("{:?}", instr_trace[7])),
                Line::from(format!("{:?}", instr_trace[8])),
                Line::from(format!("{:?}", instr_trace[9])),
                Line::from(format!("{:?}", instr_trace[10])),
                Line::from(format!("{:?}", instr_trace[11])),
                Line::from(format!("{:?}", instr_trace[12])),
                Line::from(format!("{:?}", instr_trace[13])),
                Line::from(format!("{:?}", instr_trace[14])),
                Line::from(format!("{:?}", instr_trace[15])),
                Line::from(format!("{:?}", instr_trace[16])),
                Line::from(format!("{:?}", instr_trace[17])),
                Line::from(format!("{:?}", instr_trace[18])),
                Line::from(format!("{:?}", instr_trace[19])),
                Line::from(format!("{:?}", size_of::<GbRs>())),
                Line::from(format!("FPS: {:?}", 1.0 / self.frame_time.as_secs_f64())),
                Line::from(format!("Emu FPS: {:?}", 1.0 / self.emu_time.as_secs_f64())),
            ]), top_right);


        frame.render_widget(OamWidget::new(&self.gb.cpu.bus.ppu), bottom_right);

        /*
        for oam in oams.get_oams_screen() {
            let tile = self.gb.cpu.bus.ppu.get_sprite_tile(oam.tile_idx().into());
            let oam_widget = OamWidget{ oams: *oam, tiles: tile };
            frame.render_widget(oam_widget, bottom_right);
        }
        */

    }

    fn handle_events(&mut self) -> io::Result<()> {
        if !event::poll(std::time::Duration::from_micros(100))? {
            return Ok(());
        }
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) => {
                let dir = match key_event.kind {
                    KeyEventKind::Press => JoypadDirection::PRESS,
                    KeyEventKind::Release => JoypadDirection::RELEASE,
                    _ => JoypadDirection::PRESS,
                };

                match key_event.code {
                    KeyCode::Char('q') => self.exit = true,
                    KeyCode::Char('w') => self.gb.cpu.bus.joypad.input(JoypadInput::UP, dir),
                    KeyCode::Char('a') => self.gb.cpu.bus.joypad.input(JoypadInput::LEFT, dir),
                    KeyCode::Char('d') => self.gb.cpu.bus.joypad.input(JoypadInput::RIGHT, dir),
                    KeyCode::Char('s') => self.gb.cpu.bus.joypad.input(JoypadInput::DOWN, dir),
                    KeyCode::Char('j') => self.gb.cpu.bus.joypad.input(JoypadInput::B, dir),
                    KeyCode::Char('k') => self.gb.cpu.bus.joypad.input(JoypadInput::A, dir),
                    KeyCode::Char('u') => self.gb.cpu.bus.joypad.input(JoypadInput::START, dir),
                    KeyCode::Char('i') => self.gb.cpu.bus.joypad.input(JoypadInput::SELECT, dir),
                    KeyCode::Char('b') => self.halt = true,
                    KeyCode::Char('c') => self.halt = false,
                    KeyCode::Char('n') => {
                        if key_event.kind == KeyEventKind::Press {
                            self.halt = true;
                            self.gb.run_frame();
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        };
        Ok(())
    }
}

fn to_color(color: u8) -> Color {
    match color {
        0 => Color::White,
        1 => Color::Gray,
        2 => Color::DarkGray,
        3 => Color::Black,
        _ => Color::Blue,
    }
}

impl<'a> Shape for GameFrame<'a> {
    fn draw(&self, painter: &mut Painter<'_, '_>) {
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let color = to_color(self.frame.buf[SCREEN_HEIGHT - y - 1][x]);
                if let Some((x, y)) = painter.get_point(x as f64, y as f64) {
                    painter.paint(x, y, color);
                }
            }
        }
    }
}


struct OamWidget<'a> {
    ppu: &'a gb_rs::ppu::PPU,
}

struct TileShape<'a> {
    tile: Tile<'a>,
}

impl<'a> OamWidget<'a> {

    fn new(ppu: &'a gb_rs::ppu::PPU) -> Self {
        Self {
            ppu,
        }
    }

}

impl Shape for TileShape<'_> {
    fn draw(&self, painter: &mut Painter<'_, '_>) {
        let oam_tile = self.tile.render();
        for y in 0..8 {
            for x in 0..8 {
                if let Some((x2, y2)) = painter.get_point(x as f64, y as f64) {
                    painter.paint(x2, y2, to_color(oam_tile[7 - y][x]));
                }
            }
        }
    }
}

impl Widget for OamWidget<'_> {

    fn render(self, area: Rect, buf: &mut Buffer) {

        let sprite_map = self.ppu.get_sprite_map();
        let oams = sprite_map.get_oams_screen();


        let vert = Layout::horizontal([Constraint::Length(8), Constraint::Fill(1)]).spacing(2);
        let [left, _right] = vert.areas(area);

        let contraints = Layout::vertical(Constraint::from_lengths(vec![8; oams.len() + 1])).spacing(2);

        let slots = contraints.split(left);

        for slot in slots.iter() {
            let canvas = Canvas::default()
                .marker(ratatui::symbols::Marker::HalfBlock)
                .paint(|ctx| {
                    let oam = oams.first();
                    if let Some(data) = oam {
                        let tile = self.ppu.get_sprite_tile(data.tile_idx().into());
                        ctx.draw(&TileShape { tile });
                    }
                })
            .x_bounds([0.0, 8 as f64])
            .y_bounds([0.0, 8 as f64]);

            canvas.render(*slot, buf);
        }

    }

}

pub fn run_tui(gb: GbRs) -> io::Result<()> {
    let mut app = App {
        counter: 0,
        exit: false,
        gb,
        halt: true,
        frame_time: Duration::from_secs(1),
        emu_time: Duration::from_secs(1),
    };

    let mut terminal = ratatui::init();
    execute!(
        terminal.backend_mut(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
    )
    .expect("Failure to enable key up events");

    //terminal.resize(ratatui::layout::Rect { x: 0, y: 0, width: 600, height: 600}).expect("Unable to resize :(");
    terminal.clear()?;
    let app_result = app.run(&mut terminal);

    execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags)
        .expect("Unable to disable key up events");

    ratatui::restore();

    app_result
}
