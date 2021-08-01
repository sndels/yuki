use glium::Surface;
use glutin::{
    dpi::LogicalSize,
    event::{ElementState, Event, KeyboardInput, MouseButton, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use std::{
    borrow::Cow,
    sync::{Arc, Mutex},
    time::Instant,
};

use super::{
    renderpasses::{find_min_max, HeatmapParams, ScaleOutput, ToneMapFilm},
    ui::{WriteEXR, UI},
    util::{exr_path, try_load_scene, write_exr},
    InitialSettings, ToneMapType,
};
use crate::{
    camera::CameraSample,
    expect,
    film::{Film, FilmSettings},
    integrators::IntegratorType,
    math::{Point2, Vec2, Vec3},
    renderer::{create_camera, Renderer},
    samplers::SamplerSettings,
    scene::{DynamicSceneParameters, Scene, SceneLoadSettings},
    yuki_error, yuki_info, yuki_trace,
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
        let mut ui_hovered = false;
        let mut renderer = Renderer::new();
        let mut status_messages: Option<Vec<String>> = None;
        let mut cursor_state = CursorState::default();

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
                        window,
                        &mut film_settings,
                        &mut sampler_settings,
                        &mut scene_params,
                        &mut scene_integrator,
                        &mut tone_map_type,
                        &mut load_settings,
                        &mut match_logical_cores,
                        &scene,
                        renderer.is_active(),
                        &status_messages,
                    );
                    render_triggered |= frame_ui.render_triggered;
                    any_item_active = frame_ui.any_item_active;
                    ui_hovered = frame_ui.ui_hovered;

                    if load_settings.path.exists() {
                        match try_load_scene(&load_settings) {
                            Ok((new_scene, new_scene_params, total_secs)) => {
                                scene = new_scene;
                                scene_params = new_scene_params;
                                status_messages =
                                    Some(vec![format!("Scene loaded in {:.2}s", total_secs)]);
                            }
                            Err(why) => {
                                yuki_error!("Scene loading failed: {}", why);
                                status_messages = Some(vec!["Scene loading failed".into()]);
                            }
                        }
                        load_settings.path.clear();
                    }

                    if render_triggered {
                        yuki_info!("main_loop: Render triggered");
                        // Make sure there is no render task running on when a new one is launched
                        // Need replace since the thread handle needs to be moved out
                        yuki_trace!("main_loop: Checking for an existing render job");

                        if renderer.has_finished_or_kill() {
                            yuki_info!("main_loop: Launching render job");
                            renderer.launch(
                                Arc::clone(&scene),
                                &scene_params,
                                Arc::clone(&film),
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

                    if let ToneMapType::Heatmap(HeatmapParams {
                        ref mut bounds,
                        channel,
                    }) = tone_map_type
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
                    yuki_trace!("main_loop: RedrawRequested took {:4.2}ms", spent_millis);

                    // Handle after draw so we have the mapped output texture
                    if let Some(output) = &frame_ui.write_exr {
                        match exr_path(&scene) {
                            Ok(path) => {
                                let (w, h, pixels) = match output {
                                    WriteEXR::Raw => {
                                        yuki_trace!("draw: Waiting for lock on film");
                                        let film = film.lock().unwrap();
                                        yuki_trace!("draw: Acquired film");

                                        let film_res = film.res();
                                        let pixels = film.pixels().clone();

                                        yuki_trace!("draw: Releasing film");
                                        (film_res.x as usize, film_res.y as usize, pixels)
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
                                    Some(vec![match write_exr(w, h, &pixels, &path) {
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
                    WindowEvent::ModifiersChanged(state) => cursor_state.ctrl_down = state.ctrl(),
                    WindowEvent::CursorEntered { .. } => cursor_state.inside = true,
                    WindowEvent::CursorLeft { .. } => cursor_state.inside = false,
                    WindowEvent::CursorMoved { position, .. } => {
                        cursor_state.position = Vec2::new(position.x, position.y);
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        if cursor_state.inside && !any_item_active && !ui_hovered {
                            // We only want to handle input if we're not on top of interacting with imgui

                            // Ctrl+LClick fires debug ray on pixel
                            if cursor_state.ctrl_down
                                && button == MouseButton::Left
                                && state == ElementState::Pressed
                            {
                                launch_debug_ray(
                                    &cursor_state,
                                    &display,
                                    &film,
                                    film_settings,
                                    &scene,
                                    &scene_params,
                                    scene_integrator,
                                );
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

struct CursorState {
    inside: bool,
    position: Vec2<f64>,
    ctrl_down: bool,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            inside: false,
            position: Vec2::from(0.0),
            ctrl_down: false,
        }
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

fn launch_debug_ray(
    cursor_state: &CursorState,
    display: &glium::Display,
    film: &Arc<Mutex<Film>>,
    film_settings: FilmSettings,
    scene: &Arc<Scene>,
    scene_params: &DynamicSceneParameters,
    scene_integrator: IntegratorType,
) {
    let window_px = cursor_state.position;
    yuki_info!(
        "main_loop: Debug ray initiated at window px ({},{})",
        window_px.x,
        window_px.y
    );

    let (film_w, film_h) = {
        yuki_trace!("get_film_res: Waiting for lock on film");
        let film = film.lock().unwrap();
        yuki_trace!("get_film_res: Acquired film");

        let Vec2 { x, y } = film.res();

        yuki_trace!("get_film_res: Releasing film");
        (x as f64, y as f64)
    };
    let film_aspect = film_w / film_h;

    let (window_w, window_h) = {
        let glutin::dpi::PhysicalSize { width, height } = display.gl_window().window().inner_size();
        (width as f64, height as f64)
    };
    let window_aspect = window_w / window_h;

    let film_px = if window_aspect < film_aspect {
        let x = film_w * (window_px.x / window_w);

        let film_scale = window_w / film_w;
        let bottom_margin = (window_h - film_h * film_scale) / 2.0;

        let y = (window_px.y - bottom_margin) / film_scale;

        Vec2::new(x, y)
    } else {
        let y = film_h * (window_px.y / window_h);

        let film_scale = window_h / film_h;
        let left_margin = (window_w - film_w * film_scale) / 2.0;

        let x = (window_px.x - left_margin) / film_scale;

        Vec2::new(x, y)
    };

    if film_px.min_comp() >= 0.0 && film_px.x < (film_w as f64) && film_px.y < (film_h as f64) {
        #[allow(clippy::cast_sign_loss)] // We check above
        let film_px = Vec2::new(film_px.x as u16, film_px.y as u16);

        yuki_info!(
            "main_loop: Launching debug ray at film px ({},{})",
            film_px.x,
            film_px.y
        );

        let camera = create_camera(scene_params, film_settings);

        // TODO: Use the active scene integrator instead, add evaluated rays as return data?
        {
            let p_film = Point2::new(film_px.x as f32, film_px.y as f32) + Vec2::new(0.5, 0.5);

            let ray = camera.ray(&CameraSample { p_film });

            let integrator = scene_integrator.instantiate();
            integrator.li(ray, scene, 0);
        }
    } else {
        yuki_info!("main_loop: Window px is outside the film");
    }
}
