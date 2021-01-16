// Adapted from imgui-rs example
use gfx::handle::{DepthStencilView, RenderTargetView};
use glutin::{
    dpi::LogicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    PossiblyCurrent, WindowedContext,
};
use imgui::{Context, FontConfig, FontSource};
use imgui_gfx_renderer::{Renderer, Shaders};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use old_school_gfx_glutin_ext::*;
use std::time::Instant;

type ColorFormat = gfx::format::Rgba8;
type DepthFormat = gfx::format::DepthStencil;

pub struct Window {
    // Context events
    event_loop: EventLoop<()>,
    // Imgui
    imgui_context: Context,
    imgui_platform: WinitPlatform,
    imgui_renderer: Renderer<ColorFormat, gfx_device_gl::Resources>,
    // Graphics context
    windowed_context: WindowedContext<PossiblyCurrent>,
    device: gfx_device_gl::Device,
    factory: gfx_device_gl::Factory,
    main_color: RenderTargetView<gfx_device_gl::Resources, ColorFormat>,
    // TODO: Do we need to keep this around?
    _main_depth: DepthStencilView<gfx_device_gl::Resources, DepthFormat>,
}

impl Window {
    pub fn new(title: &str, resolution: (u32, u32)) -> Window {
        let event_loop = EventLoop::new();
        let builder = WindowBuilder::new()
            .with_title(title.to_owned())
            .with_inner_size(LogicalSize::new(resolution.0 as f64, resolution.1 as f64));

        let mut imgui_context = Context::create();
        imgui_context.set_ini_filename(None);

        let mut imgui_platform = WinitPlatform::init(&mut imgui_context);

        // Setup imgui
        let hidpi_factor = imgui_platform.hidpi_factor();
        let font_size = (13.0 * hidpi_factor) as f32;
        imgui_context
            .fonts()
            .add_font(&[FontSource::DefaultFontData {
                config: Some(FontConfig {
                    size_pixels: font_size,
                    ..FontConfig::default()
                }),
            }]);

        imgui_context.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        // Init UI rendering
        {
            // Fix incorrect colors with sRGB framebuffer
            fn imgui_gamma_to_linear(col: [f32; 4]) -> [f32; 4] {
                let x = col[0].powf(2.2);
                let y = col[1].powf(2.2);
                let z = col[2].powf(2.2);
                let w = 1.0 - (1.0 - col[3]).powf(2.2);
                [x, y, z, w]
            }

            let style = imgui_context.style_mut();
            for col in 0..style.colors.len() {
                style.colors[col] = imgui_gamma_to_linear(style.colors[col]);
            }
        }

        let (windowed_context, device, mut factory, main_color, main_depth) =
            glutin::ContextBuilder::new()
                .with_vsync(true)
                .with_gfx_color_depth::<ColorFormat, DepthFormat>()
                .build_windowed(builder, &event_loop)
                .expect("Failed to initialize glutin context")
                .init_gfx::<ColorFormat, DepthFormat>();

        let shaders = {
            let version = device.get_info().shading_language;
            if version.is_embedded {
                if version.major >= 3 {
                    Shaders::GlSlEs300
                } else {
                    Shaders::GlSlEs100
                }
            } else if version.major >= 4 {
                Shaders::GlSl400
            } else if version.major >= 3 {
                if version.minor >= 2 {
                    Shaders::GlSl150
                } else {
                    Shaders::GlSl130
                }
            } else {
                Shaders::GlSl110
            }
        };

        let imgui_renderer = Renderer::init(&mut imgui_context, &mut factory, shaders)
            .expect("Failed to initialize renderer");

        imgui_platform.attach_window(
            imgui_context.io_mut(),
            windowed_context.window(),
            HiDpiMode::Rounded,
        );

        Window {
            event_loop,
            imgui_context,
            imgui_platform,
            imgui_renderer,
            windowed_context,
            device,
            factory,
            main_color,
            _main_depth: main_depth,
        }
    }

    pub fn main_loop(self) {
        let Window {
            event_loop,
            mut imgui_context,
            mut imgui_platform,
            mut imgui_renderer,
            windowed_context,
            mut device,
            mut factory,
            mut main_color,
            ..
        } = self;
        let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

        let mut last_frame = Instant::now();
        event_loop.run(move |event, _, control_flow| {
            let window = windowed_context.window();

            imgui_platform.handle_event(imgui_context.io_mut(), window, &event);
            match event {
                Event::NewEvents(_) => {
                    let now = Instant::now();
                    imgui_context.io_mut().update_delta_time(now - last_frame);
                    last_frame = now;
                }
                Event::MainEventsCleared => {
                    // Ran out of events so let's prepare to draw
                    imgui_platform
                        .prepare_frame(imgui_context.io_mut(), window)
                        .expect("Failed to prepare GL frame");
                    window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    // Init imgui for frame UI
                    let ui = imgui_context.frame();

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
                    encoder.clear(&mut main_color, [0.0, 0.0, 0.0, 0.0]);

                    // UI
                    imgui_platform.prepare_render(&ui, window);
                    imgui_renderer
                        .render(&mut factory, &mut encoder, &mut main_color, ui.render())
                        .expect("Rendering GL window failed");

                    // Finish frame
                    encoder.flush(&mut device);
                    windowed_context
                        .swap_buffers()
                        .expect("Swap buffers failed");
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(key),
                                ..
                            },
                        ..
                    } => match key {
                        VirtualKeyCode::Escape => *control_flow = ControlFlow::Exit,
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            }
        })
    }
}
