use gb_rs::cpu::Cpu;
use std::env;
use std::io;


const HORIZ_TILES: usize = 32;
const VERT_TILES: usize = 32;

const WIDTH: u32 = (HORIZ_TILES * 8) as u32;
const HEIGHT: u32 = (VERT_TILES * 8) as u32;

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
        //Path::new("roms/testrom-cpuinstr-04.gb")
    } else {
        Path::new(&args[1])
    };

    let rom = std::fs::read(path).expect("Unable to load rom file");
    let mut cpu = Cpu::new(rom.as_slice())?;

    /*
    loop {
        //cpu.log_state();
        let next_instr = cpu.next_instr();
        let clks = cpu.execute_instr(next_instr);
        if clks == 0 {
            break;
        }

        if cpu.is_passed() {
            break;
        }
    }
    */

    gui(cpu);

    Ok(())
}

fn gui(mut gb: Cpu) {

    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(WIDTH as f64, HEIGHT as f64);
        WindowBuilder::new()
            .with_title("Hello Pixels")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
        Pixels::new(WIDTH, HEIGHT, surface_texture).expect("Failed to make new pixels??")
    };


    event_loop.run(move |event, _, control_flow| {
        //let background = ppu.get_background();
        //let background = ppu.dump_vram();


        // Draw the current frame
        if let Event::RedrawRequested(_) = event {
            for _ in 0..5000 {
                gb.run_one();
            }

            let background = gb.bus.ppu.get_background();

            let mut tile_renderer = gb_rs::tile::TileRenderer::from_tiles(&background, WIDTH as usize);

            for (_, eight_pixels) in pixels.frame_mut().chunks_exact_mut(4 * 8).enumerate() {

                if let Some(new_pixels) = tile_renderer.next() {
                    for i in 0..8 {
                        eight_pixels[(4*i)..((4*i)+4)].copy_from_slice(&gb_rs::ppu::PPU::palette_to_rgba(new_pixels[i]));
                    }
                }
            }

            pixels.render().expect("Failed to render??");
        }

        // Handle input events
        if input.update(&event) {
            // Close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.close_requested() {
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
