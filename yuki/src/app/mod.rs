mod defines;
mod ui;
// Adapted from gfx-rs examples

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
use old_school_gfx_glutin_ext::*;
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use self::{
    defines::{DepthFormat, FilmFormat, FilmSurface, FilmTextureHandle, OutputColorFormat},
    ui::UI,
};
use crate::{
    expect,
    film::{Film, FilmSettings},
    integrators::IntegratorType,
    math::{Vec2, Vec3},
    renderer::Renderer,
    samplers::SamplerSettings,
    scene::{DynamicSceneParameters, Scene, SceneLoadSettings},
    yuki_debug, yuki_error, yuki_info, yuki_trace,
};

// We need to convert our Vec3<f32> pixel buffer to &[f32]
unsafe impl<T> gfx::memory::Pod for Vec3<T> where T: crate::math::ValueType {}

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
        exposure: gfx::Global<f32> = "Exposure",
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
uniform float Exposure;

in vec2 FragUV;

out vec4 OutputColor;

#define saturate(v) clamp(v, 0, 1)

// ACES implementation ported from MJP and David Neubelt's hlsl adaptation of Stephen Hill's fit
// https://github.com/TheRealMJP/BakingLab/blob/master/BakingLab/ACES.hlsl
const mat3 ACESInputMat = transpose(mat3(
    vec3(0.59719f, 0.35458f, 0.04823f),
    vec3(0.07600f, 0.90834f, 0.01566f),
    vec3(0.02840f, 0.13383f, 0.83777f)
));

// ODT_SAT => XYZ => D60_2_D65 => sRGB
const mat3 ACESOutputMat = transpose(mat3(
    vec3( 1.60475f, -0.53108f, -0.07367f),
    vec3(-0.10208f,  1.10813f, -0.00605f),
    vec3(-0.00327f, -0.07276f,  1.07602f)
));

vec3 RRTAndODTFit(vec3 v)
{
    vec3 a = v * (v + 0.0245786f) - 0.000090537f;
    vec3 b = v * (0.983729f * v + 0.4329510f) + 0.238081f;
    return a / b;
}

vec3 ACESFitted(vec3 color)
{
    color = ACESInputMat * color;

    // Apply RRT and ODT
    color = RRTAndODTFit(color);

    color = ACESOutputMat * color;

    // Clamp to [0, 1]
    color = saturate(color);

    return color;
}

void main() {
    vec3 color = texture(FilmColor, FragUV).rgb;
    color *= Exposure;
    color = ACESFitted(color);
    // Output target is linear, hw does gamma correction
    OutputColor = vec4(color, 1.0f);
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

    ui: UI,

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

        let ui = UI::new(&windowed_context, &mut factory);

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
            gfx::texture::FilterMethod::Bilinear,
            gfx::texture::WrapMode::Clamp,
        ));

        let draw_params = pipe::Data {
            vbuf: film_vbo.clone(),
            film_color: (film_view, film_sampler),
            exposure: 1.0,
            out_color: main_color,
        };

        let (scene, scene_params) = Scene::cornell();

        Window {
            event_loop,
            windowed_context,
            device,
            factory,
            main_depth,
            ui,
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
            mut ui,
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
        let mut renderer = Renderer::new();
        let mut update_film_vbo = true;
        let mut status_messages: Option<Vec<String>> = None;
        let mut load_settings = SceneLoadSettings::default();
        let mut sampler_settings = SamplerSettings::StratifiedSampler {
            pixel_samples: Vec2::new(1, 1),
            symmetric_dimensions: true,
            jitter_samples: false,
        };
        let mut exposure = 1.0;
        let mut scene_integrator = IntegratorType::Whitted;

        let mut match_logical_cores = true;

        event_loop.run(move |event, _, control_flow| {
            let window = windowed_context.window();

            ui.handle_event(window, &event);
            match event {
                Event::NewEvents(_) => {
                    yuki_trace!("main_loop: NewEvents");
                    let now = Instant::now();
                    ui.update_delta_time(now - last_frame);
                    last_frame = now;
                }
                Event::MainEventsCleared => {
                    yuki_trace!("main_loop: MainEventsCleared");
                    // Ran out of events so let's prepare to draw
                    window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    let redraw_start = Instant::now();
                    yuki_trace!("main_loop: RedrawRequested");
                    if renderer.is_active() {
                        let film_ref_count = Arc::strong_count(&film);
                        let mut messages = Vec::new();
                        if film_ref_count > 1 {
                            messages
                                .push(format!("Render threads running: {}", film_ref_count - 2));
                        }
                        if renderer.is_winding_down() {
                            messages.push("Render winding down".into());
                        }
                        status_messages = Some(messages);
                    }

                    // Run frame logic
                    let mut frame_ui = ui.generate_frame(
                        &window,
                        &mut film_settings,
                        &mut exposure,
                        &mut sampler_settings,
                        &mut scene_params,
                        &mut scene_integrator,
                        &mut load_settings,
                        &mut match_logical_cores,
                        scene.clone(),
                        &status_messages,
                    );
                    render_triggered |= frame_ui.render_triggered;
                    let new_scene_path = frame_ui.scene_path.clone();
                    any_item_active = frame_ui.any_item_active;

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

                        if renderer.has_finished_or_kill() {
                            yuki_info!("main_loop: Launching render job");
                            renderer.launch(
                                scene.clone(),
                                &scene_params,
                                film.clone(),
                                sampler_settings,
                                scene_integrator,
                                film_settings,
                                match_logical_cores,
                            );
                            render_triggered = false;
                        }
                    } else {
                        yuki_trace!("main_loop: Render job tracked");
                        if let Some(result) = renderer.check_result() {
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

                    draw_params.exposure = exposure;

                    // Draw frame
                    encoder.clear(&mut draw_params.out_color, [0.0, 0.0, 0.0, 0.0]);

                    encoder.draw(&film_ibo, &film_pso, &draw_params);

                    // UI
                    frame_ui.end_frame(
                        &window,
                        &mut factory,
                        &mut encoder,
                        &mut draw_params.out_color,
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
