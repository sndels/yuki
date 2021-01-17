// Adapted from imgui-rs gfx example and gfx-rs's examples

// Need to import gfx for macros
use gfx;
use gfx::{
    gfx_defines, gfx_impl_struct_meta, gfx_pipeline, gfx_pipeline_inner, gfx_vertex_struct_meta,
    handle::{DepthStencilView, RenderTargetView},
    traits::{Factory, FactoryExt},
};
use glutin::{
    dpi::LogicalSize,
    event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    PossiblyCurrent, WindowedContext,
};
use imgui::{im_str, FontConfig, FontSource, ImStr};
use imgui_gfx_renderer::{Renderer, Shaders};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use old_school_gfx_glutin_ext::*;
use std::{ops::DerefMut, time::Instant};

type FilmSurface = gfx::format::R32_G32_B32;
type FilmFormat = (FilmSurface, gfx::format::Float);
type OutputColorFormat = gfx::format::Rgba8;
type DepthFormat = gfx::format::DepthStencil;
type FilmTextureHandle = gfx::handle::Texture<gfx_device_gl::Resources, FilmSurface>;

use crate::{
    expect,
    film::{Film, FilmPixels, FilmSettings},
    math::{
        point::Point2,
        vector::{Vec2, Vec3},
    },
};

// We need to convert our Vec3<f32> pixel buffer to &[f32]
unsafe impl<T> gfx::memory::Pod for Vec3<T> where T: crate::math::common::ValueType {}

impl gfx::format::SurfaceTyped for Vec3<f32> {
    type DataType = Self;
    fn get_surface_type() -> gfx::format::SurfaceType {
        gfx::format::SurfaceType::R32_G32_B32
    }
}

// Simple pipeline that draws our scaled quad for film scaling and tonemapping
gfx_defines! {
    vertex Vertex {
        pos: [f32; 2] = "VertPos",
        uv: [f32; 2] = "VertUV",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        film_color: gfx::TextureSampler<[f32; 3]> = "FilmColor",
        out_color: gfx::RenderTarget<OutputColorFormat> = "OutputColor",
    }
}

const VS_CODE: &[u8] = b"#version 410 core

in vec2 VertPos;
in vec2 VertUV;

out vec2 FragUV;

void main() {
    FragUV = VertUV;
    gl_Position = vec4(VertPos, 0, 1);
}
";

const FS_CODE: &[u8] = b"#version 410 core

uniform sampler2D FilmColor;

in vec2 FragUV;

out vec4 OutputColor;

void main() {
    OutputColor = texture(FilmColor, FragUV);
}
";

pub struct Window {
    // Window
    event_loop: EventLoop<()>,
    windowed_context: WindowedContext<PossiblyCurrent>,

    // GL context
    device: gfx_device_gl::Device,
    factory: gfx_device_gl::Factory,
    main_color: RenderTargetView<gfx_device_gl::Resources, OutputColorFormat>,
    main_depth: DepthStencilView<gfx_device_gl::Resources, DepthFormat>,

    // Imgui
    imgui_context: imgui::Context,
    imgui_platform: WinitPlatform,
    imgui_renderer: Renderer<OutputColorFormat, gfx_device_gl::Resources>,

    // Rendering
    film_settings: FilmSettings,
    film: Film,
    clear_color: Vec3<f32>,

    // Film draw
    film_texture: FilmTextureHandle,
    film_pso: gfx::PipelineState<gfx_device_gl::Resources, pipe::Meta>,
}

const MIN_TILE: u16 = 2;
const MIN_RES: u16 = 64;
const MAX_RES: u16 = 4096;
const RES_STEP: u16 = 2;
const TILE_STEP: u16 = 2;

impl Window {
    pub fn new(title: &str, resolution: (u16, u16)) -> Window {
        // Create window
        let event_loop = EventLoop::new();
        let builder = WindowBuilder::new()
            .with_title(title.to_owned())
            .with_inner_size(LogicalSize::new(resolution.0 as f64, resolution.1 as f64));

        // Create gl context
        let (windowed_context, device, mut factory, main_color, main_depth) = expect!(
            glutin::ContextBuilder::new()
                .with_gl(glutin::GlRequest::Specific(glutin::Api::OpenGl, (4, 1)))
                .with_vsync(true)
                .with_gfx_color_depth::<OutputColorFormat, DepthFormat>()
                .build_windowed(builder, &event_loop),
            "Failed to initialize glutin context"
        )
        .init_gfx::<OutputColorFormat, DepthFormat>();

        // Setup imgui
        let mut imgui_context = imgui::Context::create();
        imgui_context.set_ini_filename(None);

        let mut imgui_platform = WinitPlatform::init(&mut imgui_context);

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

        let imgui_renderer = expect!(
            Renderer::init(&mut imgui_context, &mut factory, Shaders::GlSl400),
            "Failed to initialize renderer"
        );

        imgui_platform.attach_window(
            imgui_context.io_mut(),
            windowed_context.window(),
            HiDpiMode::Rounded,
        );

        let film_settings = FilmSettings::default();

        let film_texture = allocate_film_texture(&mut factory, film_settings.res);

        let shader_set = expect!(
            factory.create_shader_set(VS_CODE, FS_CODE),
            "Failed to create shader set"
        );

        let film_pso = expect!(
            factory.create_pipeline_state(
                &shader_set,
                gfx::Primitive::TriangleList,
                gfx::state::Rasterizer::new_fill(),
                pipe::new(),
            ),
            "Failed to create pso"
        );

        Window {
            event_loop,
            windowed_context,
            device,
            factory,
            main_color,
            main_depth,
            imgui_context,
            imgui_platform,
            imgui_renderer,
            film_settings,
            film: Film::default(),
            clear_color: Vec3::zeros(),
            film_texture,
            film_pso,
        }
    }

    pub fn main_loop(self) {
        let Window {
            event_loop,
            windowed_context,
            mut device,
            mut factory,
            mut main_color,
            mut main_depth,
            mut imgui_context,
            mut imgui_platform,
            mut imgui_renderer,
            mut film_settings,
            mut film,
            mut clear_color,
            film_pso,
            mut film_texture,
            ..
        } = self;
        let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

        let mut last_frame = Instant::now();
        let mut render_triggered = false;
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
                    expect!(
                        imgui_platform.prepare_frame(imgui_context.io_mut(), window),
                        "Failed to prepare GL frame"
                    );
                    window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    // Init imgui for frame UI
                    let ui = imgui_context.frame();

                    // Run frame logic

                    generate_ui(
                        &ui,
                        &mut film_settings,
                        &mut clear_color,
                        &mut render_triggered,
                    );

                    if render_triggered {
                        launch_render(&mut film, &film_settings, clear_color);
                        render_triggered = false;
                    }

                    update_texture(&mut encoder, &mut factory, &mut film_texture, &mut film);

                    let (film_indices, film_params) = create_film_draw_parameters(
                        &mut factory,
                        &window,
                        &main_color,
                        &film_texture,
                        &film,
                    );

                    // Draw frame
                    encoder.clear(&mut main_color, [0.0, 0.0, 0.0, 0.0]);

                    encoder.draw(&film_indices, &film_pso, &film_params);

                    // UI
                    imgui_platform.prepare_render(&ui, window);
                    expect!(
                        imgui_renderer.render(
                            &mut factory,
                            &mut encoder,
                            &mut main_color,
                            ui.render(),
                        ),
                        "Rendering GL window failed"
                    );

                    // Finish frame
                    encoder.flush(&mut device);
                    expect!(windowed_context.swap_buffers(), "Swap buffers failed");
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(size) => {
                        windowed_context.resize(size);
                        windowed_context.update_gfx(&mut main_color, &mut main_depth);
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(key),
                                ..
                            },
                        ..
                    } => match key {
                        VirtualKeyCode::Escape => *control_flow = ControlFlow::Exit,
                        VirtualKeyCode::Return => render_triggered = true,
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            }
        })
    }
}

fn allocate_film_texture(
    factory: &mut gfx_device_gl::Factory,
    res: Vec2<u16>,
) -> FilmTextureHandle {
    let kind = gfx::texture::Kind::D2(res.x, res.y, gfx::texture::AaMode::Single);
    expect!(
        factory.create_texture::<FilmSurface>(
            kind,
            1,
            gfx::memory::Bind::SHADER_RESOURCE | gfx::memory::Bind::TRANSFER_DST,
            gfx::memory::Usage::Dynamic,
            Some(gfx::format::ChannelType::Float),
        ),
        "Failed to create film texture"
    )
}

fn u16_picker(ui: &imgui::Ui, label: &ImStr, v: &mut u16, min: u16, max: u16, speed: f32) {
    let mut vi = *v as i32;
    imgui::Drag::new(label)
        .range((min as i32)..=(max as i32))
        .flags(imgui::SliderFlags::ALWAYS_CLAMP)
        .speed(speed)
        .build(ui, &mut vi);
    *v = vi as u16;
}

fn vec2_u16_picker(
    ui: &imgui::Ui,
    label: &ImStr,
    v: &mut Vec2<u16>,
    min: u16,
    max: u16,
    speed: f32,
) {
    let mut vi = [v.x as i32, v.y as i32];
    imgui::Drag::new(label)
        .range((min as i32)..=(max as i32))
        .flags(imgui::SliderFlags::ALWAYS_CLAMP)
        .speed(speed)
        .build_array(ui, &mut vi);
    v.x = vi[0] as u16;
    v.y = vi[1] as u16;
}

fn create_film_quad<F, R>(
    factory: &mut F,
    window_res: Vec2<u16>,
    film_res: Vec2<u16>,
) -> (gfx::handle::Buffer<R, Vertex>, gfx::Slice<R>)
where
    F: gfx::Factory<R>,
    R: gfx::Resources,
{
    // Retain film aspect ratio by scaling quad vertices directly
    let window_aspect = (window_res.x as f32) / (window_res.y as f32);
    let film_aspect = (film_res.x as f32) / (film_res.y as f32);
    let (left, right, top, bottom) = if window_aspect < film_aspect {
        let left = -1.0;
        let right = 1.0;
        let scale_y = window_aspect / film_aspect;
        let top = scale_y;
        let bottom = -scale_y;
        (left, right, top, bottom)
    } else {
        let top = 1.0;
        let bottom = -1.0;
        let scale_x = film_aspect / window_aspect;
        let left = -scale_x;
        let right = scale_x;
        (left, right, top, bottom)
    };

    let quad = [
        Vertex {
            pos: [left, bottom],
            uv: [0.0, 1.0],
        },
        Vertex {
            pos: [right, bottom],
            uv: [1.0, 1.0],
        },
        Vertex {
            pos: [right, top],
            uv: [1.0, 0.0],
        },
        Vertex {
            pos: [left, top],
            uv: [0.0, 0.0],
        },
    ];
    factory.create_vertex_buffer_with_slice(&quad, &[0u16, 1, 2, 0, 2, 3] as &[u16])
}

fn generate_ui(
    ui: &imgui::Ui,
    film_settings: &mut FilmSettings,
    clear_color: &mut Vec3<f32>,
    render_triggered: &mut bool,
) {
    imgui::Window::new(im_str!("Settings"))
        .size([325.0, 370.0], imgui::Condition::FirstUseEver)
        .build(ui, || {
            vec2_u16_picker(
                ui,
                im_str!("Resolution"),
                &mut film_settings.res,
                MIN_RES,
                MAX_RES,
                RES_STEP as f32,
            );
            u16_picker(
                ui,
                im_str!("Tile size"),
                &mut film_settings.tile_dim,
                MIN_TILE,
                MIN_RES,
                TILE_STEP as f32,
            );
            imgui::ColorPicker::new(
                im_str!("Clear color"),
                imgui::EditableColor::Float3(clear_color.array_mut()),
            )
            .flags(imgui::ColorEditFlags::PICKER_HUE_WHEEL)
            .build(ui);
            *render_triggered |= ui.button(im_str!("Render"), [50.0, 20.0]);
        });
}

fn launch_render(film: &mut Film, film_settings: &FilmSettings, clear_color: Vec3<f32>) {
    let mut tiles = film.tiles(&film_settings);
    film.clear(Vec3::new(0.0, 0.0, 0.0));

    let film_res = film.res();
    for tile in &mut tiles {
        for p in tile.bb {
            let Point2 {
                x: film_x,
                y: film_y,
            } = p;

            // Checker board pattern alternating between thread groups
            let checker_size = film_settings.tile_dim;
            let checker_quad_size = checker_size * 2;
            let mut color = if ((film_x % checker_quad_size) <= checker_size)
                ^ ((film_y % checker_quad_size) <= checker_size)
            {
                Vec3::ones()
            } else {
                clear_color
            };
            if film_y < film_res.y / 2 {
                color.y = 1.0 - color.y;
            }
            if film_x < film_res.x / 2 {
                color.x = 1.0 - color.x;
            }

            let Vec2 {
                x: tile_x,
                y: tile_y,
            } = p - tile.bb.p_min;
            tile.pixels[tile_y as usize][tile_x as usize] = color;
        }
        film.update_tile(&tile);
    }
}

fn update_texture(
    encoder: &mut gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
    factory: &mut gfx_device_gl::Factory,
    film_texture: &mut gfx::handle::Texture<gfx_device_gl::Resources, FilmSurface>,
    film: &mut Film,
) {
    // Scope lock on pixels
    // Acquire pixels already so that dirty is up to date
    let film_res = film.res();
    let film_pixels = film.pixels();
    let mut pixels_lock = expect!(film_pixels.lock(), "Failed to acquire lock on film pixels");
    let FilmPixels {
        ref pixels,
        ref mut dirty,
    } = pixels_lock.deref_mut();
    if *dirty {
        // Resize if needed
        let (tex_width, tex_height, _, _) = film_texture.get_info().kind.get_dimensions();
        if film_res.x != tex_width || film_res.y != tex_height {
            *film_texture = allocate_film_texture(factory, film_res);
        }

        // We want to update the whole thing
        // TODO: Benefit from updating partially?
        let new_info = film_texture.get_info().to_image_info(0);
        let data = gfx::memory::cast_slice(&pixels);
        expect!(
            encoder.update_texture::<_, FilmFormat>(&film_texture, None, new_info, data,),
            "Error updating film texture"
        );

        *dirty = false;
    }
}

fn create_film_draw_parameters<F, R>(
    factory: &mut F,
    window: &glutin::window::Window,
    main_color: &RenderTargetView<R, OutputColorFormat>,
    film_texture: &gfx::handle::Texture<R, gfx::format::R32_G32_B32>,
    film: &Film,
) -> (gfx::Slice<R>, pipe::Data<R>)
where
    R: gfx::Resources,
    F: gfx::Factory<R>,
{
    let film_view = expect!(
        factory.view_texture_as_shader_resource::<FilmFormat>(
            film_texture,
            (0, 0),
            gfx::format::Swizzle::new(),
        ),
        "Failed to create film shader resource view"
    );
    let film_sampler = factory.create_sampler(gfx::texture::SamplerInfo::new(
        gfx::texture::FilterMethod::Scale,
        gfx::texture::WrapMode::Clamp,
    ));

    let glutin::dpi::PhysicalSize {
        width: window_width,
        height: window_height,
    } = window.inner_size();

    let (film_vertices, film_indices) = create_film_quad(
        factory,
        Vec2::new(window_width as u16, window_height as u16),
        film.res(),
    );

    let film_params = pipe::Data {
        vbuf: film_vertices.clone(),
        film_color: (film_view, film_sampler),
        out_color: main_color.clone(),
    };

    (film_indices, film_params)
}
