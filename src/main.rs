use gb_rs::cpu::Cpu;
use std::env;
use std::io;

const WIDTH: u32 = 160;
const HEIGHT: u32 = 144;

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
        Path::new("roms/testrom-cpuinstr-05.gb")
    } else {
        Path::new(&args[1])
    };

    let rom = std::fs::read(path).expect("Unable to load rom file");
    let mut cpu = Cpu::new(rom.as_slice())?;

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

    gui();

    Ok(())
}

fn gui() {
    let test_dump = Path::new("roms/bgbtest.dump");
    let rom = std::fs::read(test_dump).expect("Unable to load test rom: {rom_path}");
    let mut ppu = gb_rs::ppu::PPU {
        vram: [0; 0x2000],
    };

    ppu.vram[0..6144].copy_from_slice(&rom);

    let vram_dump = ppu.dump_vram();

    for tile in &vram_dump {
        println!("{:?}", tile);
    }

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

    let def_tile = gb_rs::tile::Tile { pixels: [[0; 8]; 8] };
    let mut tilemap  = gb_rs::tile::TileMap { tiles: [def_tile; 384] };
    tilemap.tiles.copy_from_slice(&vram_dump[0..384]);
    


    event_loop.run(move |event, _, control_flow| {
        let mut pixel_cnt = 0;
        let mut tile_cnt = 0;

        let mut line_index = 0;
        let mut line_iter = gb_rs::tile::LineIter::from_tilemap(&tilemap, line_index);

        // Draw the current frame
        if let Event::RedrawRequested(_) = event {
            for (i, pixel) in pixels.frame_mut().chunks_exact_mut(4).enumerate() {

                let a = match line_iter.next() {
                    None => { 
                        line_index += 1;
                        /*
                        if line_index >= 100 {
                            line_index = 0;
                        }
                        */
                        line_iter = gb_rs::tile::LineIter::from_tilemap(&tilemap, line_index);
                        line_iter.next().unwrap()
                    }
                    Some(b) => { b }
                };

                //let a = vram_dump[tile_cnt].pixels[pixel_cnt / 8][pixel_cnt % 8];
                let val = 255 - (85 * a);

                let rgba = [val, val, val, 0xff];
                //let rgba = [255 , 255 , 255 , 0xff];
                //println!("A: {:?}", vram_dump[tile_cnt]);
                pixel.copy_from_slice(&rgba);
                pixel_cnt += 1;
                if pixel_cnt > 63 {
                    pixel_cnt = 0;
                    tile_cnt += 1;
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
