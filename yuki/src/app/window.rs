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

use super::{
    renderpasses::{find_min_max, ScaleOutput, ToneMapFilm},
    ui::{WriteEXR, UI},
    util::{exr_path, try_load_scene, write_exr},
    InitialSettings, ToneMapType,
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
    scene_integrator: IntegratorType,
    sampler_settings: SamplerSettings,

    // Output
    tone_map_type: ToneMapType,
    tone_map_film: ToneMapFilm,

    // Scene
    load_settings: SceneLoadSettings,
    scene: Arc<Scene>,
    scene_params: DynamicSceneParameters,
}

impl Window {
    pub fn new(title: &str, resolution: (u16, u16), settings: InitialSettings) -> Window {
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

        let tone_map_film = expect!(
            ToneMapFilm::new(&display),
            "Failed to create tone map render pass"
        );

        // Init with cornell here so scene is loaded on first frame and ui gets load time through the normal logic
        let (scene, scene_params, _) = Scene::cornell();

        Window {
            event_loop,
            display,
            ui,
            tone_map_film,
            film_settings: settings.film_settings,
            scene_integrator: settings.scene_integrator,
            sampler_settings: settings.sampler_settings,
            film,
            scene: Arc::new(scene),
            tone_map_type: settings.tone_map,
            load_settings: settings.load_settings,
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
            mut scene_integrator,
            mut sampler_settings,
            film,
            mut scene,
            mut tone_map_type,
            mut load_settings,
            mut scene_params,
        } = self;

        let mut last_frame = Instant::now();

        let mut render_triggered = false;
        let mut any_item_active = false;
        let mut renderer = Renderer::new();
        let mut status_messages: Option<Vec<String>> = None;

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
                    any_item_active = frame_ui.any_item_active;

                    if load_settings.path.exists() {
                        match try_load_scene(&load_settings) {
                            Ok((new_scene, new_scene_params, total_secs)) => {
                                scene = new_scene;
                                scene_params = new_scene_params;
                                status_messages =
                                    Some(vec![format!("Scene loaded in {:.2}s", total_secs)]);
                                load_settings.path.clear();
                            }
                            Err(why) => {
                                yuki_error!("Scene loading failed: {}", why);
                                status_messages = Some(vec!["Scene loading failed".into()]);
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

                    if let ToneMapType::Heatmap {
                        ref mut bounds,
                        channel,
                    } = tone_map_type
                    {
                        let film_dirty = {
                            yuki_trace!("main_loop: Waiting for lock on film");
                            let film = film.lock().unwrap();
                            yuki_trace!("main_loop: Aqcuired film");
                            let dirty = film.dirty();
                            yuki_trace!("main_loop: Releasing film");
                            dirty
                        };
                        if bounds.is_none() || film_dirty {
                            *bounds = Some(expect!(
                                find_min_max(&film, channel),
                                "Failed to find film min, max"
                            ));
                        }
                    }

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

                                status_messages =
                                    Some(vec![match write_exr(w, h, &pixels, path) {
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
