mod widget;

use widget::{Background, BkWindow, GameFrame, SpritesWidget, GameWidget};

use gb_rs::{
    gb::GbRs,
    joypad::{JoypadDirection, JoypadInput},
    ppu::{BKG_WIDTH, SCREEN_HEIGHT, SCREEN_WIDTH},
    tile::Tile,
    util::VecCart,
};
use std::{
    io,
    time::{Duration, Instant},
};

use crossterm::{
    event::{
        self, Event, KeyCode, KeyEventKind, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
        PushKeyboardEnhancementFlags,
    },
    execute,
};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Color,
    text::Line,
    widgets::{
        canvas::{Canvas, Painter, Shape},
        Block, Paragraph, Widget,
    },
    DefaultTerminal, Frame,
};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    rom: String,
}

enum Tab {
    MAIN,
    DEBUG,
    TILES,
    BKG,
    WINDOW,
}

pub struct App {
    counter: u32,
    halt: bool,
    exit: bool,
    gb: GbRs<VecCart>,
    draw_time: Duration,
    emu_time: Duration,
    last_frame: Instant,
    tab: u8,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            if !self.halt {
                let emu_before = Instant::now();
                self.gb.run_frame();
                self.emu_time = Instant::now().duration_since(emu_before);
            }
            let draw_before = Instant::now();
            terminal.draw(|frame| self.draw(frame))?;
            self.draw_time = Instant::now().duration_since(draw_before);
            self.handle_events()?;
            self.counter += 1;

            /* Frame rate caps -> 60fps */
            let min_frame_time = Duration::from_micros(16666);

            /* Spin here since sleeps are not accurate */
            while Instant::now().duration_since(self.last_frame) < min_frame_time {}
            self.last_frame = Instant::now();
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let horizontal =
            Layout::horizontal([Constraint::Length(SCREEN_WIDTH as u16), Constraint::Fill(1)]);
        let [left, right] = horizontal.areas(frame.area());

        let sub_vert = Layout::vertical(Constraint::from_percentages([50, 50]));

        let vertical = Layout::vertical([
            Constraint::Length(SCREEN_HEIGHT as u16 / 2),
            Constraint::Fill(1),
        ]);
        let [main, bot_left] = vertical.areas(left);

        let [top_right, bottom_right] = sub_vert.areas(right);
        let info_border = Block::bordered().title("Emulator Info");
        frame.render_widget(info_border.clone(), top_right);
        let top_right = info_border.inner(top_right);

        if self.tab == 1 {
            let game_widget = GameWidget(&self.gb.cpu.bus.ppu);
            frame.render_widget(game_widget, main);
        } else if self.tab == 2 {
            let canvas = Canvas::default()
                //.block(Block::bordered())
                .marker(ratatui::symbols::Marker::HalfBlock)
                .paint(|ctx| {
                    let bkgr = Background(&self.gb.cpu.bus.ppu);
                    ctx.draw(&bkgr);
                })
                .x_bounds([0.0, BKG_WIDTH as f64])
                .y_bounds([0.0, BKG_WIDTH as f64]);

            frame.render_widget(canvas, main);
        } else {
            let canvas = Canvas::default()
                //.block(Block::bordered())
                .marker(ratatui::symbols::Marker::HalfBlock)
                .paint(|ctx| {
                    let window = BkWindow(&self.gb.cpu.bus.ppu);
                    ctx.draw(&window);
                })
                .x_bounds([0.0, BKG_WIDTH as f64])
                .y_bounds([0.0, BKG_WIDTH as f64]);

            frame.render_widget(canvas, main);
        }

        let joypad_state = self.gb.cpu.bus.joypad.get_state();
        let ppu_state = self.gb.cpu.bus.ppu.get_ppu_state();
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(format!("{}", joypad_state)),
                Line::from(format!("SCX: {:?}", ppu_state.scx)),
                Line::from(format!("SCY: {:?}", ppu_state.scy)),
                Line::from(format!("WX: {:?}", ppu_state.wx)),
                Line::from(format!("WY: {:?}", ppu_state.wy)),
                Line::from(format!("Window Counter: {:?}", ppu_state.window_counter)),
                Line::from(format!("LY: {:?}", ppu_state.ly)),
                Line::from(format!("LYC: {:?}", ppu_state.lyc)),
                Line::from(format!("MODE: {:?}", ppu_state.mode)),
                Line::from(format!("STAT: {:?}", ppu_state.stat)),
            ]),
            bot_left,
        );

        let fps = |d: Duration| { 1.0 / d.as_secs_f64() };

        frame.render_widget(
            Paragraph::new(vec![
                Line::from(format!("GbRs Size: {:?}", size_of_val(&self.gb))),
                Line::from(format!("Emu Time: {}us ({:.1} fps)", self.emu_time.as_micros(), fps(self.emu_time))),
                Line::from(format!("Draw Time: {}us ({:.1} fps)", self.draw_time.as_micros(), fps(self.draw_time))),
                Line::from(format!("Game Title: {:?}", self.gb.cpu.bus.cart.get_header().title)),
            ]),
            top_right,
        );

        frame.render_widget(SpritesWidget(&self.gb.cpu.bus.ppu), bottom_right);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if !event::poll(std::time::Duration::from_micros(10))? {
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
                    KeyCode::Char('1') => self.tab = 1,
                    KeyCode::Char('2') => self.tab = 2,
                    KeyCode::Char('3') => self.tab = 3,
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
                    KeyCode::Char('f') => {
                        if key_event.kind == KeyEventKind::Press {
                            self.halt = true;
                            self.gb.run_frame();
                        }
                    }
                    KeyCode::Char('n') => {
                        if key_event.kind == KeyEventKind::Press {
                            self.halt = true;
                            self.gb.run_one();
                        }
                    }
                    KeyCode::Char('l') => {
                        if key_event.kind == KeyEventKind::Press {
                            self.halt = true;
                            self.gb.run_line();
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

fn run_tui(gb: GbRs<VecCart>) -> io::Result<()> {
    let mut app = App {
        counter: 0,
        exit: false,
        gb,
        halt: true,
        draw_time: Duration::from_secs(1),
        emu_time: Duration::from_secs(1),
        last_frame: Instant::now(),
        tab: 1,
    };

    let mut terminal = ratatui::init();
    execute!(
        terminal.backend_mut(),
        PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
    )
    .expect("Failure to enable key up events");

    terminal.clear()?;
    let app_result = app.run(&mut terminal);

    execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags)
        .expect("Unable to disable key up events");

    ratatui::restore();

    app_result
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();

    let rom_path = std::path::Path::new(&args.rom);
    let rom = std::fs::read(rom_path)?;

    let rom = VecCart::from_slice(&rom, Some("savedgames/"));

    let gb = GbRs::new(rom);

    run_tui(gb)?;

    Ok(())
}
