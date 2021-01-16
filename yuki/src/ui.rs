// From imgui-rs example

use glium::glutin;
use glium::glutin::event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use glium::glutin::event_loop::{ControlFlow, EventLoop};
use glium::glutin::window::WindowBuilder;
use glium::{Display, Surface};
use imgui::{Context, FontConfig, FontSource};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::path::Path;
use std::time::Instant;

pub struct Window {
    event_loop: EventLoop<()>,
    display: glium::Display,
    imgui: Context,
    platform: WinitPlatform,
    renderer: Renderer,
}

impl Window {
    pub fn new(title: &str, resolution: (u32, u32)) -> Window {
        let title = match Path::new(&title).file_name() {
            Some(file_name) => file_name.to_str().unwrap(),
            None => title,
        };
        let event_loop = EventLoop::new();
        let context = glutin::ContextBuilder::new().with_vsync(true);
        let builder = WindowBuilder::new()
            .with_title(title.to_owned())
            .with_inner_size(glutin::dpi::LogicalSize::new(
                resolution.0 as f64,
                resolution.1 as f64,
            ));
        let display =
            Display::new(builder, context, &event_loop).expect("Failed to initialize GL display");

        let mut imgui = Context::create();
        imgui.set_ini_filename(None);

        let mut platform = WinitPlatform::init(&mut imgui);
        {
            let gl_window = display.gl_window();
            let window = gl_window.window();
            platform.attach_window(imgui.io_mut(), window, HiDpiMode::Rounded);
        }

        let hidpi_factor = platform.hidpi_factor();
        let font_size = (13.0 * hidpi_factor) as f32;
        imgui.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(FontConfig {
                size_pixels: font_size,
                ..FontConfig::default()
            }),
        }]);

        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        let renderer =
            Renderer::init(&mut imgui, &display).expect("Failed to initialize GL renderer");

        Window {
            event_loop,
            display,
            imgui,
            platform,
            renderer,
        }
    }

    pub fn main_loop(self) {
        let Window {
            event_loop,
            display,
            mut imgui,
            mut platform,
            mut renderer,
            ..
        } = self;
        let mut last_frame = Instant::now();

        event_loop.run(move |event, _, control_flow| match event {
            Event::NewEvents(_) => {
                let now = Instant::now();
                imgui.io_mut().update_delta_time(now - last_frame);
                last_frame = now;
            }
            Event::MainEventsCleared => {
                let gl_window = display.gl_window();
                platform
                    .prepare_frame(imgui.io_mut(), gl_window.window())
                    .expect("Failed to prepare GL frame");
                gl_window.window().request_redraw();
            }
            Event::RedrawRequested(_) => {
                // Init imgui for frame UI
                let ui = imgui.frame();

                // Run frame logic

                // TODO: Settings UI

                // TODO: Render control

                // TODO: Get film

                // TODO: Blit film
                // TODO: Instead of film blit, render as quad and add tonemapping through frag

                imgui::Window::new(imgui::im_str!("Hello world"))
                    .size([300.0, 100.0], imgui::Condition::FirstUseEver)
                    .build(&ui, || {
                        ui.text(imgui::im_str!("Hello world!"));
                        ui.text(imgui::im_str!("こんにちは世界！"));
                        ui.text(imgui::im_str!("This...is...imgui-rs!"));
                        ui.separator();
                        let mouse_pos = ui.io().mouse_pos;
                        ui.text(format!(
                            "Mouse Position: ({:.1},{:.1})",
                            mouse_pos[0], mouse_pos[1]
                        ));
                    });

                // Draw frame
                let gl_window = display.gl_window();
                let mut target = display.draw();
                target.clear_color_srgb(0.0, 0.0, 0.0, 0.0);

                // Draw UI
                platform.prepare_render(&ui, gl_window.window());
                let draw_data = ui.render();
                renderer
                    .render(&mut target, draw_data)
                    .expect("Rendering GL window failed");

                // Finish frame
                target.finish().expect("Failed to swap GL buffers");
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(key),
                                ..
                            },
                        ..
                    },
                ..
            } => match key {
                VirtualKeyCode::Escape => *control_flow = ControlFlow::Exit,
                _ => {}
            },
            event => {
                let gl_window = display.gl_window();
                platform.handle_event(imgui.io_mut(), gl_window.window(), &event);
            }
        })
    }
}
