use gb_rs::gb::GbRs;
use std::io;

use std::io::stdout;

use ratatui::{backend::CrosstermBackend, Terminal, TerminalOptions, Viewport};


use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Stylize},
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget, canvas::{Canvas, Shape, Painter}},
    DefaultTerminal, Frame,
};

pub struct App {
    counter: u32,
    exit: bool,
    gb: GbRs
}


impl App {

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
            self.gb.run_frame();
            self.counter += 1;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        let canvas = Canvas::default()
            .block(Block::bordered().title("World, Frame:".to_string() + &self.counter.to_string()))
            .marker(ratatui::symbols::Marker::HalfBlock)
            .paint(|ctx| {
                ctx.draw(self);
            })
            .x_bounds([-100.0, 100.0])
            .y_bounds([-100.0, 100.0]);

        frame.render_widget(canvas, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if !event::poll(std::time::Duration::from_micros(100))? {
            return Ok(())
        }
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            },
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => { self.exit = true },
            _ => {}
        }
    }



}

impl Shape for App {
    fn draw(&self, painter: &mut Painter<'_, '_>) {
        let frame = self.gb.cpu.bus.ppu.background.buf;
        for y in 0..144 {
            for x in 0..160 {
                painter.paint(x, y, match frame[y][x] {
                    0 => Color::White,
                    1 => Color::Gray,
                    2 => Color::DarkGray,
                    3 => Color::Black,
                    _ => Color::Blue,
                });
            }
        }
    }
}


pub fn run_tui(gb: GbRs) -> io::Result<()> {

    let mut app = App {
        counter: 0,
        exit: false,
        gb
    };

    let backend = CrosstermBackend::new(stdout());
    let viewport = Viewport::Fixed(Rect::new(0, 0, 500, 500));
    let terminal = Terminal::with_options(backend, TerminalOptions { viewport })?;

    let mut terminal = ratatui::init();
    terminal.resize(ratatui::layout::Rect { x: 0, y: 0, width: 600, height: 600});
    terminal.clear()?;
    let app_result = app.run(&mut terminal);
    ratatui::restore();
    app_result


}



