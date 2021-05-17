mod ui;

use glium::Surface;
use glutin::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
    time::Instant,
};

use self::ui::UI;
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

impl<'a> glium::texture::Texture2dDataSource<'a> for &'a Film {
    type Data = Vec3<f32>;

    fn into_raw(self) -> glium::texture::RawImage2d<'a, Vec3<f32>> {
        let Vec2 { x, y } = self.res();
        glium::texture::RawImage2d {
            data: Cow::from(self.pixels()),
            width: x as u32,
            height: y as u32,
            format: glium::texture::ClientFormat::F32F32F32,
        }
    }
}

#[derive(Copy, Clone)]
struct FilmVertex {
    position: [f32; 2],
    uv: [f32; 2],
}

glium::implement_vertex!(FilmVertex, position, uv);

const FILM_FORMAT: glium::texture::UncompressedFloatFormat =
    glium::texture::UncompressedFloatFormat::F32F32F32;

const VS_CODE: &'static str = r#"
#version 410 core

in vec2 position;
in vec2 uv;

out vec2 frag_uv;

void main() {
    frag_uv = uv;
    gl_Position = vec4(position, 0, 1);
}
"#;

const FS_CODE: &'static str = r#"
#version 410 core

uniform sampler2D film_color;
uniform float exposure;

in vec2 frag_uv;

out vec4 output_color;

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
    vec3 color = texture(film_color, frag_uv).rgb;
    color *= exposure;
    color = ACESFitted(color);
    // Output target is linear, hw does gamma correction
    output_color = vec4(color, 1.0f);
}
"#;

pub struct Window {
    // Window and GL context
    event_loop: EventLoop<()>,
    display: glium::Display,

    ui: UI,

    // Rendering
    film_settings: FilmSettings,
    film: Arc<Mutex<Film>>,

    // Film draw
    film_vbo: glium::VertexBuffer<FilmVertex>,
    film_ibo: glium::IndexBuffer<u16>,
    film_program: glium::Program,
    film_texture: glium::Texture2d,

    // Scene
    scene: Arc<Scene>,
    scene_params: DynamicSceneParameters,
}

impl Window {
    pub fn new(title: &str, resolution: (u16, u16)) -> Window {
        // Create window and gl context
        let event_loop = EventLoop::new();
        let window_builder = WindowBuilder::new()
            .with_title(title.to_owned())
            .with_inner_size(LogicalSize::new(resolution.0 as f64, resolution.1 as f64));
        let context_builder = glutin::ContextBuilder::new();
        let display = expect!(
            glium::Display::new(window_builder, context_builder, &event_loop),
            "Failed to initialize glium display"
        );

        let ui = UI::new(&display);

        // Film
        let film = Arc::new(Mutex::new(Film::default()));

        let film_settings = FilmSettings::default();

        // Film draw
        let film_vbo = create_film_vbo(&display, &film);

        let film_ibo = expect!(
            glium::index::IndexBuffer::new(
                &display,
                glium::index::PrimitiveType::TrianglesList,
                &[0u16, 1, 2, 0, 2, 3]
            ),
            "Failed to create film index buffer"
        );
        let film_program = expect!(
            glium::Program::from_source(&display, VS_CODE, FS_CODE, None),
            "Failed to create film "
        );

        let film_texture = expect!(
            glium::Texture2d::empty_with_format(
                &display,
                FILM_FORMAT,
                glium::texture::MipmapsOption::NoMipmap,
                film_settings.res.x as u32,
                film_settings.res.y as u32,
            ),
            "Failed to create film texture"
        );

        let (scene, scene_params) = Scene::cornell();

        Window {
            event_loop,
            display,
            ui,
            film_settings,
            film,
            film_vbo,
            film_ibo,
            film_program,
            film_texture,
            scene: Arc::new(scene),
            scene_params,
        }
    }

    pub fn main_loop(self) {
        let Window {
            event_loop,
            display,
            mut ui,
            mut film_settings,
            film,
            mut film_vbo,
            film_ibo,
            film_program,
            mut film_texture,
            mut scene,
            mut scene_params,
        } = self;

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
            let gl_window = display.gl_window();
            let window = gl_window.window();

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
                    if update_film_texture(&display, &film, &mut film_texture) {
                        yuki_debug!("main_loop: Texture size changed, updating vbo");
                        update_film_vbo = true;
                    }

                    if update_film_vbo {
                        yuki_debug!("main_loop: VBO update required");
                        film_vbo = create_film_vbo(&display, &film);
                    }

                    // Draw frame
                    let mut render_target = display.draw();
                    render_target.clear_color_srgb(0.0, 0.0, 0.0, 0.0);

                    let uniforms = glium::uniform! {
                            film_color: film_texture.sampled()
                                .wrap_function(glium::uniforms::SamplerWrapFunction::BorderClamp)
                                .minify_filter(glium::uniforms::MinifySamplerFilter::Linear)
                                .magnify_filter(glium::uniforms::MagnifySamplerFilter::Linear),
                            exposure: exposure,
                    };
                    expect!(
                        render_target.draw(
                            &film_vbo,
                            &film_ibo,
                            &film_program,
                            &uniforms,
                            &Default::default()
                        ),
                        "Failed to draw film"
                    );

                    // UI
                    frame_ui.end_frame(&display, &mut render_target);

                    // Finish frame
                    expect!(render_target.finish(), "Frame::finish() failed");

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
                        display.gl_window().resize(size);
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

fn update_film_texture(
    display: &glium::Display,
    film: &Mutex<Film>,
    texture: &mut glium::Texture2d,
) -> bool {
    let mut resized = false;

    yuki_trace!("create_film_texture: Begin");
    yuki_trace!("create_film_texture: Waiting for lock on film");
    let mut film = film.lock().unwrap();
    yuki_trace!("create_film_texture: Acquired film");

    if film.dirty() {
        yuki_debug!("create_film_texture: Film is dirty");
        // We could update only the tiles that have changed but that's more work and scaffolding
        // than it's worth especially with marked tiles. This is fast enough at small resolutions.
        *texture = expect!(
            glium::Texture2d::with_format(
                display,
                &*film,
                FILM_FORMAT,
                glium::texture::MipmapsOption::NoMipmap
            ),
            "Error creating new film texture"
        );
        let film_res = film.res();
        resized = film_res.x != (texture.width() as u16) || film_res.y != (texture.height() as u16);

        film.clear_dirty();
        yuki_trace!("create_film_texture: Texture created");
    }

    yuki_trace!("create_film_texture: Releasing film");
    resized
}

fn create_film_vbo(
    display: &glium::Display,
    film: &Mutex<Film>,
) -> glium::VertexBuffer<FilmVertex> {
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
    } = display.gl_window().window().inner_size();

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
        FilmVertex {
            position: [left, bottom],
            uv: [0.0, 1.0],
        },
        FilmVertex {
            position: [right, bottom],
            uv: [1.0, 1.0],
        },
        FilmVertex {
            position: [right, top],
            uv: [1.0, 0.0],
        },
        FilmVertex {
            position: [left, top],
            uv: [0.0, 0.0],
        },
    ];
    expect!(
        glium::VertexBuffer::new(display, &quad),
        "Failed to create film vertex buffer"
    )
}
