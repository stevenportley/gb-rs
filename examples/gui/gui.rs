use gb_rs::{gb::GbRs, rom::Rom};
use pixels::wgpu;
use std::time::Instant;

use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::Fullscreen;
use winit::window::Window;
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

const WIDTH: u32 = 160;
const SCALING: f64 = 4.0;
const HEIGHT: u32 = 144;

/// Manages all state required for rendering Dear ImGui over `Pixels`.
pub(crate) struct Gui {
    gb: GbRs,
    event_loop: EventLoop<()>,
    pixels: Pixels,

    window: Window,

    imgui: imgui::Context,
    platform: imgui_winit_support::WinitPlatform,
    renderer: imgui_wgpu::Renderer,
    last_frame: Instant,
    last_cursor: Option<imgui::MouseCursor>,
    about_open: bool,
    metrics_window: bool,
}

impl Gui {
    pub fn new(gb: GbRs) -> Self {
        let event_loop = EventLoop::new();
        let window = {
            let size = LogicalSize::new(SCALING * WIDTH as f64, SCALING * HEIGHT as f64);
            WindowBuilder::new()
                .with_title("Hello Pixels")
                .with_inner_size(size)
                .with_min_inner_size(size)
                .with_fullscreen(Some(Fullscreen::Borderless(None))) /* The GUI crashes on my macbook without starting the GUI in full screen mode? */
                .build(&event_loop)
                .unwrap()
        };

        //TODO: What was this for??
        //let mut scale_factor = window.scale_factor();

        // Create Dear ImGui context
        let mut imgui = imgui::Context::create();
        imgui.set_ini_filename(None);

        // Initialize winit platform support
        let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
        platform.attach_window(
            imgui.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Default,
        );

        let pixels = {
            let window_size = window.inner_size();
            let surface_texture =
                SurfaceTexture::new(window_size.width, window_size.height, &window);
            Pixels::new(WIDTH, HEIGHT, surface_texture).expect("Failed to make new pixels??")
        };

        // Create Dear ImGui WGPU renderer
        let device = pixels.device();
        let queue = pixels.queue();
        let config = imgui_wgpu::RendererConfig {
            texture_format: pixels.render_texture_format(),
            ..Default::default()
        };
        let renderer = imgui_wgpu::Renderer::new(&mut imgui, device, queue, config);

        // Return GUI context
        Self {
            gb,
            event_loop,
            imgui,
            pixels,
            window,
            platform,
            renderer,
            last_frame: Instant::now(),
            last_cursor: None,
            about_open: true,
            metrics_window: false,
        }
    }

    /// Render Dear ImGui.
    pub(crate) fn render(
        ui: &mut imgui::Ui,
        about_open: &mut bool,
        metrics_window: &mut bool,
    ) -> imgui_wgpu::RendererResult<()> {
        // Draw windows and GUI elements here
        let mut about_open2 = false;
        let mut metrics_window2 = false;
        ui.main_menu_bar(|| {
            ui.menu("Help", || {
                about_open2 = ui.menu_item("About...");
            });

            ui.menu("Metrics", || {
                metrics_window2 = ui.menu_item("Metrics...");
            });
        });
        if about_open2 {
            *about_open = true;
        }

        if metrics_window2 {
            *metrics_window = true;
        }

        if *about_open {
            ui.show_about_window(about_open);
        }

        if *metrics_window {
            ui.show_metrics_window(metrics_window);
            ui.window("Example Window")
                .size([100.0, 50.0], imgui::Condition::FirstUseEver)
                .build(|| {
                    ui.text("An example");
                });
        }

        Ok(())
    }

    pub fn run(mut self) {
        let mut input = WinitInputHelper::new();

        self.event_loop.run(move |event, _, control_flow| {
            // Draw the current frame
            if let Event::RedrawRequested(_) = event {
                let frame = self.gb.cpu.bus.ppu.get_screen();
                self.pixels.frame_mut()[..frame.len()].copy_from_slice(&frame);

                // Prepare Dear ImGui
                let now = Instant::now();
                self.imgui.io_mut().update_delta_time(now - self.last_frame);
                self.last_frame = now;
                let _ = self
                    .platform
                    .prepare_frame(self.imgui.io_mut(), &self.window);

                let render_result = self.pixels.render_with(|encoder, render_target, context| {
                    context.scaling_renderer.render(encoder, render_target);
                    // Start a new Dear ImGui frame and update the cursor
                    let ui = self.imgui.new_frame();

                    let mouse_cursor = ui.mouse_cursor();
                    if self.last_cursor != mouse_cursor {
                        self.last_cursor = mouse_cursor;
                        self.platform.prepare_render(ui, &self.window);
                    }
                    Self::render(ui, &mut self.about_open, &mut self.metrics_window)?;

                    // Render Dear ImGui with WGPU
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("imgui"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: render_target,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });
                    self.renderer.render(
                        self.imgui.render(),
                        &context.queue,
                        &context.device,
                        &mut rpass,
                    )?;

                    Ok(())
                });

                if let Err(err) = render_result {
                    println!("pixels.render: {}", err);
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }

            self.platform
                .handle_event(self.imgui.io_mut(), &self.window, &event);

            // Handle input events
            if input.update(&event) {
                // Close events
                if input.key_pressed(VirtualKeyCode::Escape) {
                    *control_flow = ControlFlow::Exit;
                    return;
                }

                // Resize the window
                if let Some(size) = input.window_resized() {
                    self.pixels
                        .resize_surface(size.width, size.height)
                        .expect("Failed to resize?");
                }

                // Resize the window
                if let Some(size) = input.window_resized() {
                    if size.width > 0 && size.height > 0 {
                        // Resize the surface texture
                        if let Err(err) = self.pixels.resize_surface(size.width, size.height) {
                            println!("pixels.resize_surface: {}", err);
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                    }
                }

                self.gb.run_frame();

                // Update internal state and request a redraw
                self.window.request_redraw();
            }
        });
    }
}

fn main() -> std::io::Result<()> {
    let rom_path = std::path::Path::new("roms/tetris.gb");
    let rom = std::fs::read(rom_path).expect("Unable to load test rom: {rom_path}");
    let gb = GbRs::new(&rom);
    let gui = Gui::new(gb);
    gui.run();

    Ok(())
}
