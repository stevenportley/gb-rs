use gb_rs::{gb::GbRs, joypad::JoypadDirection, joypad::JoypadInput};
use ratatui::buffer::Cell;
use ratatui::widgets::canvas::Context;
use std::io;

use std::io::stdout;

use ratatui::{backend::CrosstermBackend, Terminal, TerminalOptions, Viewport};
use ratatui::layout::{Position, Constraint, Layout};


use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::execute;
use crossterm::event::{
    KeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags
};


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

struct GameFrame<'a> {
    frame: &'a gb_rs::ppu::Frame,
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

    fn draw(&mut self, frame: &mut Frame) {

        let canvas = Canvas::default()
            //.block(Block::bordered())
            .marker(ratatui::symbols::Marker::HalfBlock)
            .paint(|ctx| {
                let game_frame = GameFrame {
                    frame: &self.gb.cpu.bus.ppu.background,
                };
                ctx.draw(&game_frame);
            })
            .x_bounds([0.0, 160.0])
            .y_bounds([0.0, 144.0]);


        let horizontal = Layout::horizontal([Constraint::Length(160), Constraint::Fill(1)]);
        let [left, right] = horizontal.areas(frame.area());

        let vertical = Layout::vertical([Constraint::Length(72), Constraint::Fill(1)]);
        let [main, _] = vertical.areas(left);

        frame.render_widget(Block::bordered()
            .title(format!("GB RS: Area: {:?}, Frame: {}", left, &self.counter.to_string())), right);
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
            ]),
            right)
        


    }

    fn handle_events(&mut self) -> io::Result<()> {
        if !event::poll(std::time::Duration::from_micros(100))? {
            return Ok(())
        }
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) => {
                let dir = match key_event.kind {
                    KeyEventKind::Press => JoypadDirection::PRESS,
                    KeyEventKind::Release => JoypadDirection::RELEASE,
                    _ =>  JoypadDirection::PRESS,
                };

                match key_event.code {
                    KeyCode::Char('q') => { self.exit = true },
                    KeyCode::Char('w') => { self.gb.cpu.bus.joypad.input(JoypadInput::UP, dir) },
                    KeyCode::Char('a') => { self.gb.cpu.bus.joypad.input(JoypadInput::LEFT, dir) },
                    KeyCode::Char('d') => { self.gb.cpu.bus.joypad.input(JoypadInput::RIGHT, dir) },
                    KeyCode::Char('s') => { self.gb.cpu.bus.joypad.input(JoypadInput::DOWN, dir) },
                    KeyCode::Char('j') => { self.gb.cpu.bus.joypad.input(JoypadInput::B, dir) },
                    KeyCode::Char('k') => { self.gb.cpu.bus.joypad.input(JoypadInput::A, dir) },
                    KeyCode::Char('u') => { self.gb.cpu.bus.joypad.input(JoypadInput::START, dir) },
                    KeyCode::Char('i') => { self.gb.cpu.bus.joypad.input(JoypadInput::SELECT, dir) },
                    _ => {}
                }
            },
            _ => {}
        };
        Ok(())
    }

}

impl<'a> Shape for GameFrame<'a> {
    fn draw(&self, painter: &mut Painter<'_, '_>) {

        for y in 0..144 {
            for x in 0..160 {
                let color = match self.frame.buf[144 - y][x] {
                    0 => Color::White,
                    1 => Color::Gray,
                    2 => Color::DarkGray,
                    3 => Color::Black,
                    _ => Color::Blue,
                };

                if let Some((x, y)) = painter.get_point(x as f64, y as f64) {
                    painter.paint(x, y, color);
                }
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

    let mut terminal = ratatui::init();
    execute!(
        terminal.backend_mut(),
        PushKeyboardEnhancementFlags(
        KeyboardEnhancementFlags::REPORT_EVENT_TYPES
        )
    ).expect("Failure to enable key up events");

    //terminal.resize(ratatui::layout::Rect { x: 0, y: 0, width: 600, height: 600}).expect("Unable to resize :(");
    terminal.clear()?;
    let app_result = app.run(&mut terminal);

    execute!(terminal.backend_mut(), PopKeyboardEnhancementFlags)
        .expect("Unable to disable key up events");

    ratatui::restore();

    app_result
}



