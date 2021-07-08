mod renderpasses;
mod ui;

use chrono::{Datelike, Timelike};
use glium::Surface;
use glutin::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use std::{
    borrow::Cow,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant,
};

use self::{
    renderpasses::{ScaleOutput, ToneMapFilm, ToneMapType},
    ui::{WriteEXR, UI},
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

pub struct Window {
    // Window and GL context
    event_loop: EventLoop<()>,
    display: glium::Display,

    ui: UI,

    // Rendering
    film_settings: FilmSettings,
    film: Arc<Mutex<Film>>,

    // Output
    tone_map_film: ToneMapFilm,

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
        // Vsync is an easy way to limit framerate to a sane range
        let context_builder = glutin::ContextBuilder::new().with_vsync(true);
        let display = expect!(
            glium::Display::new(window_builder, context_builder, &event_loop),
            "Failed to initialize glium display"
        );

        let ui = UI::new(&display);

        // Film
        let film = Arc::new(Mutex::new(Film::default()));

        let film_settings = FilmSettings::default();

        let tone_map_film = expect!(
            ToneMapFilm::new(&display),
            "Failed to create tone map render pass"
        );

        let (scene, scene_params) = Scene::cornell();

        Window {
            event_loop,
            display,
            ui,
            tone_map_film,
            film_settings,
            film,
            scene: Arc::new(scene),
            scene_params,
        }
    }

    pub fn main_loop(self) {
        let Window {
            event_loop,
            display,
            mut ui,
            mut tone_map_film,
            mut film_settings,
            film,
            mut scene,
            mut scene_params,
        } = self;

        let mut last_frame = Instant::now();

        let mut render_triggered = false;
        let mut any_item_active = false;
        let mut renderer = Renderer::new();
        let mut status_messages: Option<Vec<String>> = None;
        let mut load_settings = SceneLoadSettings::default();
        let mut sampler_settings = SamplerSettings::StratifiedSampler {
            pixel_samples: Vec2::new(1, 1),
            symmetric_dimensions: true,
            jitter_samples: false,
        };
        let mut scene_integrator = IntegratorType::Whitted;
        let mut tone_map_type = ToneMapType::Filmic { exposure: 1.0 };

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
                        &mut sampler_settings,
                        &mut scene_params,
                        &mut scene_integrator,
                        &mut tone_map_type,
                        &mut load_settings,
                        &mut match_logical_cores,
                        scene.clone(),
                        renderer.is_active(),
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

                    // Draw frame
                    let mut render_target = display.draw();
                    render_target.clear_color_srgb(0.0, 0.0, 0.0, 0.0);

                    let tone_mapped_film = expect!(
                        tone_map_film.draw(&display, &film, &tone_map_type),
                        "Film tone map pass failed"
                    );
                    ScaleOutput::draw(tone_mapped_film, &mut render_target);

                    // UI
                    frame_ui.end_frame(&display, &mut render_target);

                    // Finish frame
                    expect!(render_target.finish(), "Frame::finish() failed");

                    let spent_millis = (redraw_start.elapsed().as_micros() as f32) * 1e-3;
                    yuki_debug!("main_loop: RedrawRequested took {:4.2}ms", spent_millis);

                    // Handle after draw so we have the mapped output texture
                    if let Some(output) = &frame_ui.write_exr {
                        match exr_path(&scene) {
                            Ok(path) => {
                                let (w, h, pixels) = match output {
                                    WriteEXR::Raw => {
                                        yuki_trace!("Write EXR: Waiting for lock on film");
                                        let film = film.lock().unwrap();
                                        yuki_trace!("Write EXR: Acquired film");

                                        let (w, h) = {
                                            let Vec2 { x, y } = film.res();
                                            (x as usize, y as usize)
                                        };

                                        let pixels = film.pixels().clone();

                                        (w, h, pixels)
                                    }

                                    WriteEXR::Mapped => {
                                        let w = tone_mapped_film.width() as usize;
                                        let h = tone_mapped_film.height() as usize;
                                        // TODO: This will explode if mapped texture format is not f32f32f32
                                        let pixels = unsafe {
                                            tone_mapped_film
                                                .unchecked_read::<Vec<Vec3<f32>>, Vec3<f32>>()
                                        };
                                        (w, h, pixels)
                                    }
                                };

                                status_messages = Some(vec![match write_exr(w, h, pixels, path) {
                                    Ok(_) => "EXR written".into(),
                                    Err(why) => {
                                        yuki_error!("{}", why);
                                        "Error writing EXR".into()
                                    }
                                }]);
                            }
                            Err(why) => {
                                yuki_error!("{}", why);
                            }
                        }
                    }
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        yuki_trace!("main_loop: CloseRequsted");
                        *control_flow = ControlFlow::Exit;
                    }
                    WindowEvent::Resized(size) => {
                        yuki_trace!("main_loop: Resized");
                        display.gl_window().resize(size);
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

fn exr_path(scene: &Scene) -> Result<PathBuf, String> {
    match std::env::current_dir() {
        Ok(mut path) => {
            let now = chrono::Local::now();
            let timestamp = format!(
                "{:04}{:02}{:02}_{:02}{:02}{:02}",
                now.year(),
                now.month(),
                now.day(),
                now.hour(),
                now.minute(),
                now.second()
            );
            let filename = format!("{}_{}.exr", scene.name, timestamp);
            path.push(filename);

            Ok(path)
        }
        Err(why) => Err(format!(
            "Error getting current working directory: {:?}",
            why
        )),
    }
}

fn write_exr(
    width: usize,
    height: usize,
    pixels: Vec<Vec3<f32>>,
    path: PathBuf,
) -> Result<(), String> {
    yuki_info!("Writing out EXR");
    match exr::prelude::write_rgb_file(&path, width, height, |x, y| {
        let px = pixels[y * width + x];
        (px.x, px.y, px.z)
    }) {
        Ok(_) => {
            yuki_info!("EXR written to '{}'", path.to_string_lossy());
            Ok(())
        }
        Err(why) => Err(format!(
            "Error writing EXR to '{}': {:?}",
            path.to_string_lossy(),
            why
        )),
    }
}

impl glium::texture::Texture2dDataSink<Vec3<f32>> for Vec<Vec3<f32>> {
    fn from_raw(data: Cow<'_, [Vec3<f32>]>, _: u32, _: u32) -> Self {
        data.into()
    }
}

unsafe impl glium::texture::PixelValue for Vec3<f32> {
    fn get_format() -> glium::texture::ClientFormat {
        glium::texture::ClientFormat::F32F32F32
    }
}
