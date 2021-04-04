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
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{
        mpsc::{channel, Receiver, Sender, TryRecvError},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::Instant,
};
use tinyfiledialogs::open_file_dialog;

type FilmSurface = gfx::format::R32_G32_B32;
type FilmFormat = (FilmSurface, gfx::format::Float);
type OutputColorFormat = gfx::format::Rgba8;
type DepthFormat = gfx::format::DepthStencil;
type FilmTextureHandle = gfx::handle::Texture<gfx_device_gl::Resources, FilmSurface>;

use crate::{
    camera::{Camera, CameraSample, FoV},
    expect,
    film::{film_tiles, Film, FilmSettings, FilmTile},
    math::{
        point::Point2,
        transform::look_at,
        vector::{Vec2, Vec3},
    },
    scene::{DynamicSceneParameters, Scene, SceneLoadSettings},
    yuki_debug, yuki_error, yuki_info, yuki_trace, yuki_warn,
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

    // Film draw
    film_pso: gfx::PipelineState<gfx_device_gl::Resources, pipe::Meta>,
    draw_params: pipe::Data<gfx_device_gl::Resources>,
    // vbo is owned by params
    film_ibo: gfx::Slice<gfx_device_gl::Resources>,
    film_texture: FilmTextureHandle,

    scene: Arc<Scene>,
    scene_params: DynamicSceneParameters,
}

const MIN_TILE: u16 = 8;
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
            fn imgui_gamma_to_linear(col: [f32; 4]) -> [f32; 4] {
                let x = col[0].powf(2.2);
                let y = col[1].powf(2.2);
                let z = col[2].powf(2.2);
                let w = 1.0 - (1.0 - col[3]).powf(2.2);
                [x, y, z, w]
            }

            let style = imgui_context.style_mut();
            // Do rectangular elements
            style.window_rounding = 0.0;
            style.child_rounding = 0.0;
            style.popup_rounding = 0.0;
            style.grab_rounding = 0.0;
            style.tab_rounding = 0.0;
            style.frame_rounding = 0.0;
            style.scrollbar_rounding = 0.0;
            // No border line
            style.window_border_size = 0.0;
            // Fix incorrect colors with sRGB framebuffer
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

        let (scene, scene_params) = Scene::cornell();

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
            film_pso,
            draw_params,
            film_ibo,
            film_texture,
            scene: Arc::new(scene),
            scene_params,
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
            film_pso,
            mut film_texture,
            mut draw_params,
            film_ibo,
            mut scene,
            mut scene_params,
            ..
        } = self;
        let mut encoder: gfx::Encoder<_, _> = factory.create_command_buffer().into();

        let mut last_frame = Instant::now();

        let mut render_triggered = false;
        let mut any_item_active = false;
        let mut render_handle: Option<(
            Option<Sender<usize>>,
            Receiver<RenderResult>,
            JoinHandle<_>,
        )> = None;
        let mut render_ending = false;
        let mut update_film_vbo = true;
        let mut status_messages: Option<Vec<String>> = None;
        let mut load_settings = SceneLoadSettings::default();

        let mut match_logical_cores = true;

        macro_rules! cleanup {
            () => {
                if let Some((to_render, _, render_thread)) =
                    std::mem::replace(&mut render_handle, None)
                {
                    if let Some(tx) = to_render {
                        let _ = tx.send(0);
                    }
                    render_thread.join().unwrap();
                }
            };
        }

        event_loop.run(move |event, _, control_flow| {
            let window = windowed_context.window();

            imgui_platform.handle_event(imgui_context.io_mut(), window, &event);
            match event {
                Event::NewEvents(_) => {
                    yuki_trace!("main_loop: NewEvents");
                    let now = Instant::now();
                    imgui_context.io_mut().update_delta_time(now - last_frame);
                    last_frame = now;
                }
                Event::MainEventsCleared => {
                    yuki_trace!("main_loop: MainEventsCleared");
                    // Ran out of events so let's prepare to draw
                    expect!(
                        imgui_platform.prepare_frame(imgui_context.io_mut(), window),
                        "Failed to prepare GL frame"
                    );
                    window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    let redraw_start = Instant::now();
                    yuki_trace!("main_loop: RedrawRequested");
                    // Init imgui for frame UI
                    let ui = imgui_context.frame();

                    if render_handle.is_some() {
                        let film_ref_count = Arc::strong_count(&film);
                        let mut messages = Vec::new();
                        if film_ref_count > 1 {
                            messages
                                .push(format!("Render threads running: {}", film_ref_count - 2));
                        }
                        if render_ending {
                            messages.push("Render winding down".into());
                        }
                        status_messages = Some(messages);
                    }

                    // Run frame logic
                    let ui_ret = generate_ui(
                        &ui,
                        &window,
                        &mut film_settings,
                        &mut scene_params,
                        &mut load_settings,
                        &mut match_logical_cores,
                        scene.clone(),
                        &status_messages,
                    );
                    render_triggered |= ui_ret.render_triggered;
                    let new_scene_path = ui_ret.scene_path;
                    any_item_active = ui.is_any_item_active();

                    if let Some(path) = new_scene_path {
                        match path.extension() {
                            Some(ext) => match ext.to_str().unwrap() {
                                "ply" => match Scene::ply(&path, load_settings) {
                                    Ok((new_scene, new_scene_params, total_secs)) => {
                                        yuki_info!(
                                            "PLY loaded from {}",
                                            path.file_name().unwrap().to_str().unwrap()
                                        );

                                        scene = Arc::new(new_scene);
                                        scene_params = new_scene_params;
                                        status_messages = Some(vec![format!(
                                            "Scene loaded in {:.2}s",
                                            total_secs
                                        )]);
                                    }
                                    Err(why) => {
                                        yuki_error!("Loading PLY failed: {}", why);
                                        status_messages = Some(vec!["Scene loading failed".into()]);
                                    }
                                },
                                "xml" => match Scene::mitsuba(&path, load_settings) {
                                    Ok((new_scene, new_scene_params, total_secs)) => {
                                        yuki_info!(
                                            "Mitsuba 2.0 scene loaded from {}",
                                            path.file_name().unwrap().to_str().unwrap()
                                        );

                                        scene = Arc::new(new_scene);
                                        scene_params = new_scene_params;
                                        status_messages = Some(vec![format!(
                                            "Scene loaded in {:.2}s",
                                            total_secs
                                        )]);
                                    }
                                    Err(why) => {
                                        yuki_error!("Loading Mitsuba 2.0 scene failed: {}", why);
                                        status_messages = Some(vec!["Scene loading failed".into()]);
                                    }
                                },
                                _ => {
                                    // TODO: Why can't this be a oneline "comma"-branch?
                                    yuki_error!("Unknown extension '{}'", ext.to_str().unwrap());
                                }
                            },
                            None => {
                                // TODO: Why can't this be a oneline "comma"-branch?
                                yuki_error!("Expected a file with an extension");
                            }
                        }
                    }

                    if render_triggered {
                        yuki_info!("main_loop: Render triggered");
                        // Make sure there is no render task running on when a new one is launched
                        // Need replace since the thread handle needs to be moved out
                        yuki_trace!("main_loop: Checking for an existing render job");
                        render_ending = check_and_kill_running_render(&mut render_handle);

                        if render_handle.is_none() {
                            yuki_info!("main_loop: Launching render job");
                            let (to_render, render_rx) = channel();
                            let (render_tx, from_render) = channel();
                            let render_thread = launch_render(
                                render_tx,
                                render_rx,
                                scene.clone(),
                                &scene_params,
                                film.clone(),
                                film_settings,
                                match_logical_cores,
                            );
                            yuki_trace!("main_loop: Render job launched");

                            render_handle = Some((Some(to_render), from_render, render_thread));
                            render_triggered = false;
                        }
                    } else {
                        yuki_trace!("main_loop: Render job tracked");
                        if let Some(result) = check_running_render(&mut render_handle) {
                            status_messages = Some(vec![
                                format!("Render finished in {:.2}s", result.secs),
                                format!(
                                    "{:.2} Mrays/s",
                                    ((result.ray_count as f32) / result.secs) * 1e-6
                                ),
                            ]);
                        }
                    }

                    yuki_trace!("main_loop: Checking for texture update");
                    if let Some(film_view) =
                        update_texture(&mut encoder, &mut factory, &mut film_texture, &film)
                    {
                        yuki_debug!("main_loop: Texture size changed, updating view");
                        draw_params.film_color.0 = film_view;
                        // Texture size changed so we need to update output scaling
                        update_film_vbo = true;
                    }

                    if update_film_vbo {
                        yuki_debug!("main_loop: VBO update required");
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

                    let spent_millis = (redraw_start.elapsed().as_micros() as f32) * 1e-3;
                    yuki_debug!("main_loop: RedrawRequested took {:4.2}ms", spent_millis);
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        yuki_trace!("main_loop: CloseRequsted");
                        cleanup!();
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::Resized(size) => {
                        yuki_trace!("main_loop: Resized");
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
                        yuki_trace!("main_loop: KeyboardInput");
                        if !any_item_active {
                            // We only want to handle keypresses if we're not interacting with imgui
                            match key {
                                VirtualKeyCode::Escape => {
                                    cleanup!();
                                    *control_flow = ControlFlow::Exit;
                                }
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

fn u16_picker(ui: &imgui::Ui, label: &ImStr, v: &mut u16, min: u16, max: u16, speed: f32) -> bool {
    let mut vi = *v as i32;

    let value_changed = imgui::Drag::new(label)
        .range((min as i32)..=(max as i32))
        .flags(imgui::SliderFlags::ALWAYS_CLAMP)
        .speed(speed)
        .build(ui, &mut vi);

    *v = vi as u16;

    value_changed
}

fn vec2_u16_picker(
    ui: &imgui::Ui,
    label: &ImStr,
    v: &mut Vec2<u16>,
    min: u16,
    max: u16,
    speed: f32,
) -> bool {
    let mut vi = [v.x as i32, v.y as i32];

    let value_changed = imgui::Drag::new(label)
        .range((min as i32)..=(max as i32))
        .flags(imgui::SliderFlags::ALWAYS_CLAMP)
        .speed(speed)
        .build_array(ui, &mut vi);

    v.x = vi[0] as u16;
    v.y = vi[1] as u16;

    value_changed
}

struct UiReturn {
    render_triggered: bool,
    scene_path: Option<PathBuf>,
}

fn generate_ui(
    ui: &imgui::Ui,
    window: &glutin::window::Window,
    film_settings: &mut FilmSettings,
    scene_params: &mut DynamicSceneParameters,
    load_settings: &mut SceneLoadSettings,
    match_logical_cores: &mut bool,
    scene: Arc<Scene>,
    status_messages: &Option<Vec<String>>,
) -> UiReturn {
    let glutin::dpi::PhysicalSize {
        width: _,
        height: window_height,
    } = window.inner_size();

    let mut ret = UiReturn {
        render_triggered: false,
        scene_path: None,
    };

    imgui::Window::new(im_str!("Settings"))
        .position([0.0, 0.0], imgui::Condition::Always)
        .size([370.0, window_height as f32], imgui::Condition::Always)
        .resizable(false)
        .movable(false)
        .build(ui, || {
            imgui::TreeNode::new(im_str!("Film"))
                .default_open(true)
                .build(ui, || {
                    ret.render_triggered |= vec2_u16_picker(
                        ui,
                        im_str!("Resolution"),
                        &mut film_settings.res,
                        MIN_RES,
                        MAX_RES,
                        RES_STEP as f32,
                    );
                    {
                        let width = ui.push_item_width(118.0);
                        ret.render_triggered |= u16_picker(
                            ui,
                            im_str!("Tile size"),
                            &mut film_settings.tile_dim,
                            MIN_TILE,
                            MIN_RES,
                            TILE_STEP as f32,
                        );
                        width.pop(ui);
                    }
                    ret.render_triggered |=
                        ui.checkbox(im_str!("Clear buffer"), &mut film_settings.clear);
                    imgui::TreeNode::new(im_str!("Clear color")).build(ui, || {
                        ret.render_triggered |= imgui::ColorPicker::new(
                            im_str!("Clear color picker"),
                            imgui::EditableColor::Float3(film_settings.clear_color.array_mut()),
                        )
                        .flags(
                            imgui::ColorEditFlags::NO_LABEL
                                | imgui::ColorEditFlags::PICKER_HUE_WHEEL,
                        )
                        .build(ui);
                    });
                });

            ui.spacing();

            imgui::TreeNode::new(im_str!("Scene"))
                .default_open(true)
                .build(ui, || {
                    imgui::TreeNode::new(im_str!("Camera"))
                        .default_open(true)
                        .build(ui, || {
                            ret.render_triggered |= imgui::Drag::new(im_str!("Position"))
                                .speed(0.1)
                                .display_format(im_str!("%.1f"))
                                .build_array(ui, scene_params.cam_pos.array_mut());

                            ret.render_triggered |= imgui::Drag::new(im_str!("Target"))
                                .speed(0.1)
                                .display_format(im_str!("%.1f"))
                                .build_array(ui, scene_params.cam_target.array_mut());

                            {
                                let width = ui.push_item_width(77.0);
                                let fov = match &mut scene_params.cam_fov {
                                    FoV::X(ref mut v) => v,
                                    FoV::Y(ref mut v) => v,
                                };
                                ret.render_triggered |= imgui::Drag::new(im_str!("Field of View"))
                                    .range(0.1..=359.9)
                                    .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                                    .speed(0.5)
                                    .display_format(im_str!("%.1f"))
                                    .build(ui, fov);
                                width.pop(ui);
                            }
                        });

                    ui.spacing();

                    {
                        let width = ui.push_item_width(92.0);
                        u16_picker(
                            ui,
                            im_str!("Max shapes in BVH node"),
                            &mut load_settings.max_shapes_in_node,
                            1,
                            u16::max_value(),
                            1.0,
                        );
                        width.pop(ui);
                    }

                    ui.spacing();

                    if ui.button(im_str!("Change scene"), [92.0, 20.0]) {
                        let open_path = if let Some(path) = &scene.path {
                            path.to_str().unwrap()
                        } else {
                            ""
                        };
                        ret.scene_path = if let Some(path) = open_file_dialog(
                            "Open scene",
                            open_path,
                            Some((&["*.ply", "*.xml"], "Supported scene formats")),
                        ) {
                            Some(PathBuf::from(path))
                        } else {
                            None
                        };
                    }
                    ui.same_line(0.0);
                    if ui.button(im_str!("Reload scene"), [92.0, 20.0]) {
                        ret.scene_path = scene.path.clone();
                    }
                });

            ui.spacing();

            ui.checkbox(im_str!("Match logical cores"), match_logical_cores);

            ui.spacing();

            ret.render_triggered |= ui.button(im_str!("Render"), [50.0, 20.0]);

            ui.spacing();
            ui.separator();

            ui.text(im_str!("Current scene: {}", scene.name));
            ui.text(im_str!("Shape count: {}", scene.geometry.len()));
            ui.text(im_str!(
                "Shapes in BVH node: {}",
                (scene.settings.max_shapes_in_node as usize).min(scene.geometry.len())
            ));

            ui.spacing();
            ui.separator();

            if let Some(lines) = status_messages {
                for l in lines {
                    ui.text(im_str!("{}", l));
                }
            }
        });

    ret
}

#[derive(Copy, Clone)]
struct RenderResult {
    secs: f32,
    ray_count: usize,
}

fn launch_render(
    to_parent: Sender<RenderResult>,
    from_parent: Receiver<usize>,
    scene: Arc<Scene>,
    scene_params: &DynamicSceneParameters,
    mut film: Arc<Mutex<Film>>,
    film_settings: FilmSettings,
    match_logical_cores: bool,
) -> JoinHandle<()> {
    let camera = Camera::new(
        &look_at(
            scene_params.cam_pos,
            scene_params.cam_target,
            Vec3::new(0.0, 1.0, 0.0),
        )
        .inverted(),
        scene_params.cam_fov,
        &film_settings,
    );

    std::thread::spawn(move || {
        yuki_debug!("Render: Begin");
        yuki_trace!("Render: Getting tiles");
        // Get tiles, resizes film if necessary
        let tiles = Arc::new(Mutex::new(film_tiles(&mut film, &film_settings)));

        yuki_trace!("Render: Launch threads");
        let render_start = Instant::now();
        let thread_count = if match_logical_cores {
            num_cpus::get()
        } else {
            num_cpus::get_physical()
        };
        let (child_send, from_children) = channel();
        let mut children: HashMap<usize, (Sender<usize>, JoinHandle<_>)> = (0..thread_count)
            .map(|i| {
                let (to_child, child_receive) = channel();
                let child_send = child_send.clone();
                let tiles = tiles.clone();
                let camera = camera.clone();
                let scene = scene.clone();
                let film = film.clone();
                (
                    i,
                    (
                        to_child,
                        std::thread::spawn(move || {
                            render(
                                i,
                                child_send,
                                child_receive,
                                tiles,
                                film_settings.clear_color,
                                scene,
                                camera,
                                film,
                            );
                        }),
                    ),
                )
            })
            .collect();

        // Wait for children to finish
        let mut ray_count = 0;
        while !children.is_empty() {
            if let Ok(_) = from_parent.try_recv() {
                yuki_debug!("Render: Killed by parent");
                break;
            }

            if let Ok((thread_id, rays)) = from_children.try_recv() {
                yuki_trace!("Render: Join {}", thread_id);
                let (_, child) = children.remove(&thread_id).unwrap();
                child.join().unwrap();
                yuki_trace!("Render: {} terminated", thread_id);
                ray_count += rays;
            } else {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }

        // Kill children after being killed
        if !children.is_empty() {
            // Kill everyone first
            for (_, (tx, _)) in &children {
                // No need to check for error, child having disconnected, since that's our goal
                let _ = tx.send(0);
            }
            // Wait for everyone to end
            for (thread_id, (_, child)) in children {
                // This message might not be from the same child, but we don't really care as long as
                // every child notifies us
                from_children.recv().unwrap();
                child.join().unwrap();
                yuki_debug!("Render: {} terminated", thread_id);
            }
        }

        yuki_trace!("Render: Report back");
        let secs = (render_start.elapsed().as_micros() as f32) * 1e-6;
        if let Err(why) = to_parent.send(RenderResult { secs, ray_count }) {
            yuki_error!("Render: Error notifying parent: {}", why);
        };
        yuki_debug!("Render: End");
    })
}

fn render(
    thread_id: usize,
    to_parent: Sender<(usize, usize)>,
    from_parent: Receiver<usize>,
    tiles: Arc<Mutex<VecDeque<FilmTile>>>,
    clear_color: Vec3<f32>,
    scene: Arc<Scene>,
    camera: Camera,
    film: Arc<Mutex<Film>>,
) {
    yuki_debug!("Render thread {}: Begin", thread_id);

    let mut rays = 0;
    'work: loop {
        if let Ok(_) = from_parent.try_recv() {
            yuki_debug!("Render thread {}: Killed by parent", thread_id);
            break 'work;
        }

        let tile = {
            let mut tiles = tiles.lock().unwrap();
            tiles.pop_front()
        };
        if tile.is_none() {
            break;
        }
        let mut tile = tile.unwrap();
        let tile_width = tile.bb.p_max.x - tile.bb.p_min.x;

        yuki_trace!("Render thread {}: Render tile {:?}", thread_id, tile.bb);
        for p in tile.bb {
            // Let's have low latency kills for more interactive view
            if let Ok(_) = from_parent.try_recv() {
                yuki_debug!("Render thread {}: Killed by parent", thread_id);
                break 'work;
            }

            let ray = camera.ray(CameraSample {
                p_film: Point2::new(p.x as f32, p.y as f32),
            });

            let hit = scene.bvh.intersect(ray);
            rays += 1;

            let color = if let Some(hit) = hit {
                // TODO: Do color/spectrum class for this math
                fn mul(v1: Vec3<f32>, v2: Vec3<f32>) -> Vec3<f32> {
                    Vec3::new(v1.x * v2.x, v1.y * v2.y, v1.z * v2.z)
                }
                let light_sample = scene.light.sample_li(&hit);
                // TODO: Trace light visibility
                mul(hit.albedo / std::f32::consts::PI, light_sample.li)
                    * hit.n.dot_v(light_sample.l).clamp(0.0, 1.0)
            } else {
                clear_color
            };

            let Vec2 {
                x: tile_x,
                y: tile_y,
            } = p - tile.bb.p_min;
            let pixel_offset = (tile_y * tile_width + tile_x) as usize;
            tile.pixels[pixel_offset] = color;
        }

        yuki_trace!("Render thread {}: Update tile {:?}", thread_id, tile.bb);
        {
            yuki_trace!("Render thread {}: Waiting for lock on film", thread_id);
            let mut film = film.lock().unwrap();
            yuki_trace!("Render thread {}: Acquired film", thread_id);

            film.update_tile(tile);

            yuki_trace!("Render thread {}: Releasing film", thread_id);
        }
    }

    yuki_trace!("Render thread {}: Signal end", thread_id);
    if let Err(why) = to_parent.send((thread_id, rays)) {
        yuki_error!("Render thread {}: Error: {}", thread_id, why);
    };
    yuki_debug!("Render thread {}: End", thread_id);
}

fn update_texture(
    encoder: &mut gfx::Encoder<gfx_device_gl::Resources, gfx_device_gl::CommandBuffer>,
    factory: &mut gfx_device_gl::Factory,
    film_texture: &mut gfx::handle::Texture<gfx_device_gl::Resources, FilmSurface>,
    film: &Mutex<Film>,
) -> Option<gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 3]>> {
    let mut ret = None;

    yuki_trace!("update_texture: Begin");
    yuki_trace!("update_texture: Waiting for lock on film");
    let mut film = film.lock().unwrap();
    yuki_trace!("update_texture: Acquired film");

    let film_res = film.res();
    if film.dirty() {
        yuki_debug!("update_texture: Film is dirty");
        let film_pixels = film.pixels();

        // Resize texture if needed
        let (tex_width, tex_height, _, _) = film_texture.get_info().kind.get_dimensions();
        if film_res.x != tex_width || film_res.y != tex_height {
            yuki_trace!("update_texture: Resizing texture");
            *film_texture = allocate_film_texture(factory, film_res);

            ret = Some(expect!(
                factory.view_texture_as_shader_resource::<FilmFormat>(
                    &film_texture,
                    (0, 0),
                    gfx::format::Swizzle::new(),
                ),
                "Failed to create film shader resource view"
            ));
            yuki_trace!("update_texture: Resized");
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
        yuki_trace!("update_texture: Texture updated");
    }

    yuki_trace!("update_texture: Releasing film");
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
        yuki_trace!("create_film_vbo: Locking film");
        let film = film.lock().unwrap();
        let res = film.res();
        yuki_trace!("create_film_vbo: Releasing film");
        res
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

fn check_and_kill_running_render(
    render_handle: &mut Option<(
        Option<Sender<usize>>,
        Receiver<RenderResult>,
        JoinHandle<()>,
    )>,
) -> bool {
    let mut render_ending = false;
    let rm = std::mem::replace(render_handle, None);
    if let Some((to_render, from_render, render_thread)) = rm {
        yuki_trace!("check_and_kill_running_render: Checking if the render job has finished");
        // See if the task has completed
        match from_render.try_recv() {
            Ok(_) => {
                yuki_trace!(
                    "check_and_kill_running_render: Waiting for the finished render job to exit"
                );
                render_thread.join().unwrap();
                yuki_debug!("check_and_kill_running_render: Render job has finished");
            }
            Err(why) => {
                // Task is either still running or has disconnected without notifying us
                match why {
                    TryRecvError::Empty => {
                        yuki_debug!("check_and_kill_running_render: Render job still running");
                        if let Some(tx) = to_render {
                            // Kill thread on first time here
                            yuki_trace!("check_and_kill_running_render: Sending kill command to the render job");
                            let _ = tx.send(0);
                        }
                        // Keep handles to continue polling until the thread has stopped
                        // We won't be sending anything after the kill command
                        *render_handle = Some((None, from_render, render_thread));
                        render_ending = true;
                    }
                    TryRecvError::Disconnected => {
                        yuki_warn!(
                            "check_and_kill_running_render: Render disconnected without notifying"
                        );
                        render_thread.join().unwrap();
                    }
                }
            }
        }
    } else {
        yuki_debug!("check_and_kill_running_render: No existing render job");
    }
    render_ending
}

fn check_running_render(
    render_handle: &mut Option<(
        Option<Sender<usize>>,
        Receiver<RenderResult>,
        JoinHandle<()>,
    )>,
) -> Option<RenderResult> {
    let mut ret = None;
    let rm = std::mem::replace(render_handle, None);
    if let Some((to_render, from_render, render_thread)) = rm {
        match from_render.try_recv() {
            Ok(result) => {
                yuki_trace!("check_running_render: Waiting for the finished render job to exit");
                render_thread.join().unwrap();
                yuki_debug!("check_running_render: Render job has finished");
                ret = Some(result);
            }
            Err(why) => match why {
                TryRecvError::Empty => {
                    yuki_debug!("check_running_render: Render job still running");
                    *render_handle = Some((to_render, from_render, render_thread));
                }
                TryRecvError::Disconnected => {
                    yuki_warn!("check_running_render: Render disconnected without notifying");
                    render_thread.join().unwrap();
                }
            },
        }
    }
    ret
}
