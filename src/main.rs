use gb_rs::gb::GbRs;
use std::env;
use std::io;
use winit::window::Fullscreen;

mod gui;

use gui::Gui;

const HORIZ_TILES: usize = 32;
const VERT_TILES: usize = 32;

const WIDTH: u32 = (HORIZ_TILES * 8) as u32;
const HEIGHT: u32 = (VERT_TILES * 8) as u32;
const SCALING: f64 = 4.0;

use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

use std::path::Path;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let path = if args.len() != 2 {
        Path::new("roms/tetris.gb")
    } else {
        Path::new(&args[1])
    };

    let rom = std::fs::read(path).expect("Unable to load rom file");
    let cpu = GbRs::new(rom.as_slice())?;

    gui(cpu);

    Ok(())
}

fn gui(mut gb: GbRs) {
    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(SCALING * WIDTH as f64, SCALING * HEIGHT as f64);
        WindowBuilder::new()
            .with_title("Hello Pixels")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .with_fullscreen(Some(Fullscreen::Borderless(None)))
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture).expect("Failed to make new pixels??")
    };

    let mut gui = Gui::new(&window, &pixels);


    event_loop.run(move |event, _, control_flow| {

        // Draw the current frame
        if let Event::RedrawRequested(_) = event {

            let cycles_per_frame = 17556;
            let mut cycles_so_far = 0;

            while cycles_so_far < cycles_per_frame {
                cycles_so_far += gb.run_one();
            }

            let frame = gb.cpu.bus.ppu.get_frame();
            pixels.frame_mut()[..(8 * 32) * (4 * 8 * 32)].copy_from_slice(&frame);

            gui.prepare(&window).expect("gui.prepare() failed");

            let render_result = pixels.render_with(|encoder, render_target, context| {
                context.scaling_renderer.render(encoder, render_target);

                gui.render(&window, encoder, render_target, context)?;

                Ok(())
            });

            if let Err(err) = render_result {
                println!("pixels.render: {}", err);
                *control_flow = ControlFlow::Exit;
                return;
            }

        }

        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) {
                *control_flow = ControlFlow::Exit;
                return;
            }

            // Resize the window
            if let Some(size) = input.window_resized() {
                pixels
                    .resize_surface(size.width, size.height)
                    .expect("Failed to resize?");
            }

            // Update internal state and request a redraw
            window.request_redraw();
        }
    });
}
