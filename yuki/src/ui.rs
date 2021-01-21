// Adapted from imgui-rs gfx example and gfx-rs's examples

// Need to import gfx for macros
use gfx;
use gfx::{
    gfx_defines, gfx_impl_struct_meta, gfx_pipeline, gfx_pipeline_inner, gfx_vertex_struct_meta,
    handle::DepthStencilView,
    traits::{Factory, FactoryExt},
};
use glutin::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
    PossiblyCurrent, WindowedContext,
};
use imgui::{im_str, FontConfig, FontSource, ImStr};
use imgui_gfx_renderer::{Renderer, Shaders};
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use old_school_gfx_glutin_ext::*;
use std::{
    collections::HashMap,
    sync::{mpsc::channel, Arc, Mutex},
    thread::JoinHandle,
    time::Instant,
};

type FilmSurface = gfx::format::R32_G32_B32;
type FilmFormat = (FilmSurface, gfx::format::Float);
type OutputColorFormat = gfx::format::Rgba8;
type DepthFormat = gfx::format::DepthStencil;
type FilmTextureHandle = gfx::handle::Texture<gfx_device_gl::Resources, FilmSurface>;

use crate::{
    camera::{Camera, CameraSample},
    expect,
    film::{Film, FilmSettings, FilmTile},
    math::{
        point::{Point2, Point3},
        transform::{look_at, translation},
        vector::{Vec2, Vec3},
    },
    sphere::Sphere,
    yuki_debug, yuki_error,
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
    // main_color is owned by draw_params
    main_depth: DepthStencilView<gfx_device_gl::Resources, DepthFormat>,

    // Imgui
    imgui_context: imgui::Context,
    imgui_platform: WinitPlatform,
    imgui_renderer: Renderer<OutputColorFormat, gfx_device_gl::Resources>,

    // Rendering
    film_settings: FilmSettings,
    film: Arc<Mutex<Film>>,
    clear_color: Vec3<f32>,

    // Film draw
    film_pso: gfx::PipelineState<gfx_device_gl::Resources, pipe::Meta>,
    draw_params: pipe::Data<gfx_device_gl::Resources>,
    // vbo is owned by params
    film_ibo: gfx::Slice<gfx_device_gl::Resources>,
    film_texture: FilmTextureHandle,

    // Scene
    scene: Arc<Sphere>,
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

        // Film
        let film = Arc::new(Mutex::new(Film::default()));

        let film_settings = FilmSettings::default();

        // Film draw
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

        let quad = [
            Vertex {
                pos: [-1.0, -1.0],
                uv: [0.0, 1.0],
            },
            Vertex {
                pos: [1.0, -1.0],
                uv: [1.0, 1.0],
            },
            Vertex {
                pos: [1.0, 1.0],
                uv: [1.0, 0.0],
            },
            Vertex {
                pos: [-1.0, 1.0],
                uv: [0.0, 0.0],
            },
        ];
        let (film_vbo, film_ibo) =
            factory.create_vertex_buffer_with_slice(&quad, &[0u16, 1, 2, 0, 2, 3] as &[u16]);

        let film_texture = allocate_film_texture(&mut factory, film_settings.res);

        let film_view = expect!(
            factory.view_texture_as_shader_resource::<FilmFormat>(
                &film_texture,
                (0, 0),
                gfx::format::Swizzle::new(),
            ),
            "Failed to create film shader resource view"
        );
        let film_sampler = factory.create_sampler(gfx::texture::SamplerInfo::new(
            gfx::texture::FilterMethod::Scale,
            gfx::texture::WrapMode::Clamp,
        ));

        let draw_params = pipe::Data {
            vbuf: film_vbo.clone(),
            film_color: (film_view, film_sampler),
            out_color: main_color,
        };

        let scene = Arc::new(Sphere::new(&translation(Vec3::new(0.0, 0.0, 0.0)), 1.0));

        Window {
            event_loop,
            windowed_context,
            device,
            factory,
            main_depth,
            imgui_context,
            imgui_platform,
            imgui_renderer,
            film_settings,
            film,
            clear_color: Vec3::zeros(),
            film_pso,
            draw_params,
            film_ibo,
            film_texture,
            scene,
        }
    }

    pub fn main_loop(self) {
        let Window {
            event_loop,
            windowed_context,
            mut device,
            mut factory,
            mut main_depth,
            mut imgui_context,
            mut imgui_platform,
            mut imgui_renderer,
            mut film_settings,
            film,
            mut clear_color,
            film_pso,
            mut film_texture,
            mut draw_params,
            film_ibo,
            scene,
            ..
        } = self;
        let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

        let mut last_frame = Instant::now();

        let mut render_triggered = false;
        let mut any_item_active = false;
        let mut render_manager: Option<JoinHandle<_>> = None;
        let mut update_film_vbo = true;

        let mut cam_pos = Point3::new(2.0, 2.0, -3.0);
        let mut cam_target = Point3::new(0.0, 0.0, 0.0);
        let mut cam_fov = 60.0;

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
                        &mut cam_pos,
                        &mut cam_target,
                        &mut cam_fov,
                    );
                    any_item_active = ui.is_any_item_active();

                    if render_triggered {
                        let rm = std::mem::replace(&mut render_manager, None);
                        if let Some(thread) = rm {
                            thread.join().unwrap();
                        }

                        // Get tiles, resizes film if necessary
                        let tiles = {
                            let mut film = film.lock().unwrap();
                            Arc::new(Mutex::new(film.tiles(&film_settings)))
                        };

                        let camera = Arc::new(Camera::new(
                            &look_at(cam_pos, cam_target, Vec3::new(0.0, 1.0, 0.0)).inverted(),
                            cam_fov,
                            &film,
                        ));

                        render_manager = Some(launch_render(
                            &camera,
                            &scene,
                            film.clone(),
                            tiles,
                            film_settings,
                            clear_color,
                        ));
                        render_triggered = false;
                    }

                    if let Some(film_view) =
                        update_texture(&mut encoder, &mut factory, &mut film_texture, &film)
                    {
                        draw_params.film_color.0 = film_view;
                        // Texture size changed so we need to update output scaling
                        update_film_vbo = true;
                    }

                    if update_film_vbo {
                        draw_params.vbuf = create_film_vbo(&mut factory, &window, &film);

                        update_film_vbo = false;
                    }

                    // Draw frame
                    encoder.clear(&mut draw_params.out_color, [0.0, 0.0, 0.0, 0.0]);

                    encoder.draw(&film_ibo, &film_pso, &draw_params);

                    // UI
                    imgui_platform.prepare_render(&ui, window);
                    expect!(
                        imgui_renderer.render(
                            &mut factory,
                            &mut encoder,
                            &mut draw_params.out_color,
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
                        windowed_context.update_gfx(&mut draw_params.out_color, &mut main_depth);

                        update_film_vbo = true;
                    }
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(key),
                                state: ElementState::Pressed,
                                ..
                            },
                        ..
                    } => {
                        if !any_item_active {
                            // We only want to handle keypresses if we're not interacting with imgui
                            match key {
                                VirtualKeyCode::Escape => *control_flow = ControlFlow::Exit,
                                VirtualKeyCode::Return => render_triggered = true,
                                _ => {}
                            }
                        }
                    }
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

fn generate_ui(
    ui: &imgui::Ui,
    film_settings: &mut FilmSettings,
    clear_color: &mut Vec3<f32>,
    render_triggered: &mut bool,
    cam_pos: &mut Point3<f32>,
    cam_target: &mut Point3<f32>,
    cam_fov: &mut f32,
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

            ui.text(im_str!("Camera"));

            imgui::Drag::new(im_str!("Position"))
                .speed(0.1)
                .display_format(im_str!("%.1f"))
                .build_array(ui, cam_pos.array_mut());

            imgui::Drag::new(im_str!("Target"))
                .speed(0.1)
                .display_format(im_str!("%.1f"))
                .build_array(ui, cam_target.array_mut());

            imgui::Drag::new(im_str!("Field of View"))
                .range(0.1..=359.9)
                .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                .speed(0.5)
                .display_format(im_str!("%.1f"))
                .build(ui, cam_fov);

            *render_triggered |= ui.button(im_str!("Render"), [50.0, 20.0]);
        });
}

fn launch_render(
    camera: &Arc<Camera>,
    scene: &Arc<Sphere>,
    film: Arc<Mutex<Film>>,
    tiles: Arc<Mutex<Vec<FilmTile>>>,
    film_settings: FilmSettings,
    clear_color: Vec3<f32>,
) -> JoinHandle<()> {
    let camera = camera.clone();
    let scene = scene.clone();

    std::thread::spawn(move || {
        yuki_debug!("Render manager: Start");
        let checker_size = film_settings.tile_dim;
        let (tx, rx) = channel();
        // TODO: Proper num based on hw?
        let mut children: HashMap<usize, JoinHandle<_>> = (0..4)
            .map(|i| {
                let tx = tx.clone();
                let tiles = tiles.clone();
                let camera = camera.clone();
                let scene = scene.clone();
                let film = film.clone();
                (
                    i,
                    std::thread::spawn(move || {
                        render(i, tx, tiles, checker_size, clear_color, camera, scene, film);
                    }),
                )
            })
            .collect();

        while !children.is_empty() {
            if let Ok(thread_id) = rx.try_recv() {
                yuki_debug!("Render manager: Join {}", thread_id);
                let child = children.remove(&thread_id).unwrap();
                child.join().unwrap();
                yuki_debug!("Render manager: {} terminated", thread_id);
            } else {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        yuki_debug!("Render manager: End");
    })
}

fn render(
    thread_id: usize,
    tx: std::sync::mpsc::Sender<usize>,
    tiles: Arc<Mutex<Vec<FilmTile>>>,
    checker_size: u16,
    clear_color: Vec3<f32>,
    camera: Arc<Camera>,
    scene: Arc<Sphere>,
    film: Arc<Mutex<Film>>,
) {
    yuki_debug!("Thread {}: Start", thread_id);

    let film_res = {
        let film = film.lock().unwrap();
        film.res()
    };

    loop {
        let tile = {
            let mut tiles = tiles.lock().unwrap();
            if !tiles.is_empty() {
                Some(tiles.pop().unwrap())
            } else {
                None
            }
        };
        if tile.is_none() {
            break;
        }
        let mut tile = tile.unwrap();

        yuki_debug!("Thread {}: Render tile {:?}", thread_id, tile.bb);
        for p in tile.bb {
            let ray = camera.ray(CameraSample {
                p_film: Point2::new(p.x as f32, p.y as f32),
            });

            let color = if let Some(t) = scene.intersect(ray) {
                Vec3::from(ray.point(t))
            } else {
                let Point2 {
                    x: film_x,
                    y: film_y,
                } = p;
                // Checker board pattern alternating between thread groups
                let checker_quad_size = checker_size * 2;
                let mut color = if ((film_x % checker_quad_size) < checker_size)
                    ^ ((film_y % checker_quad_size) < checker_size)
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
                color
            };

            let Vec2 {
                x: tile_x,
                y: tile_y,
            } = p - tile.bb.p_min;
            tile.pixels[tile_y as usize][tile_x as usize] = color;
        }

        yuki_debug!("Thread {}: Update tile {:?}", thread_id, tile.bb);
        {
            let mut film = film.lock().unwrap();
            film.update_tile(tile);
        }
    }
    yuki_debug!("Thread {}: Signal end", thread_id);
    if let Err(why) = tx.send(thread_id) {
        yuki_error!("Thread {} error: {}", thread_id, why);
    };
    yuki_debug!("Thread {}: End", thread_id);
}

fn update_texture(
    encoder: &mut gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
    factory: &mut gfx_device_gl::Factory,
    film_texture: &mut gfx::handle::Texture<gfx_device_gl::Resources, FilmSurface>,
    film: &Mutex<Film>,
) -> Option<gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 3]>> {
    let mut ret = None;
    let mut film = film.lock().unwrap();
    let film_res = film.res();
    if film.dirty() {
        let film_pixels = film.pixels();

        // Resize texture if needed
        let (tex_width, tex_height, _, _) = film_texture.get_info().kind.get_dimensions();
        if film_res.x != tex_width || film_res.y != tex_height {
            *film_texture = allocate_film_texture(factory, film_res);

            ret = Some(expect!(
                factory.view_texture_as_shader_resource::<FilmFormat>(
                    &film_texture,
                    (0, 0),
                    gfx::format::Swizzle::new(),
                ),
                "Failed to create film shader resource view"
            ));
        }

        // Update texture
        // TODO: Benefit from updating partially?
        let new_info = film_texture.get_info().to_image_info(0);
        let data = gfx::memory::cast_slice(&film_pixels);
        expect!(
            encoder.update_texture::<_, FilmFormat>(&film_texture, None, new_info, data,),
            "Error updating film texture"
        );

        film.clear_dirty();
    }
    ret
}

fn create_film_vbo<F, R>(
    factory: &mut F,
    window: &glutin::window::Window,
    film: &Mutex<Film>,
) -> gfx::handle::Buffer<R, Vertex>
where
    R: gfx::Resources,
    F: gfx::Factory<R>,
{
    let film_res = {
        let film = film.lock().unwrap();
        film.res()
    };

    let glutin::dpi::PhysicalSize {
        width: window_width,
        height: window_height,
    } = window.inner_size();

    // Retain film aspect ratio by scaling quad vertices directly
    let window_aspect = (window_width as f32) / (window_height as f32);
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
    factory.create_vertex_buffer(&quad)
}
