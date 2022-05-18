use allocators::{LinearAllocator, ScopedScratch};
use approx::relative_ne;
use glium::glutin;
use glium::{
    glutin::{
        dpi::LogicalSize,
        event::{
            ElementState, Event, KeyboardInput, MouseButton, MouseScrollDelta, VirtualKeyCode,
            WindowEvent,
        },
        event_loop::{ControlFlow, EventLoop},
        platform::run_return::EventLoopExtRunReturn,
        window::WindowBuilder,
    },
    Surface,
};
use std::{
    borrow::Cow,
    fs::File,
    io::BufWriter,
    sync::{Arc, Mutex},
    time::Instant,
};

use super::{
    renderpasses::{
        find_min_max, BvhVisualization, HeatmapParams, RayVisualization, ScaleOutput, ToneMapFilm,
    },
    ui::{generate_ui, WriteEXR, UI},
    util::{exr_path, try_load_scene, write_exr},
    InitialSettings, ToneMapType,
};
use crate::{
    camera::{Camera, CameraParameters, CameraSample, FoV},
    expect,
    film::{film_or_new, Film, FilmSettings},
    integrators::{IntegratorRay, IntegratorType},
    math::{transforms::rotation, Point2, Spectrum, Vec2, Vec3},
    renderer::{RenderSettings, RenderStatus, Renderer},
    sampling::Sampler,
    sampling::SamplerType,
    scene::{Scene, SceneLoadSettings},
    yuki_debug, yuki_error, yuki_info, yuki_trace,
};

pub struct Window {
    // Window and GL context
    event_loop: EventLoop<()>,
    display: glium::Display,

    ui: UI,

    // Rendering
    film_settings: FilmSettings,
    render_settings: RenderSettings,
    film: Arc<Mutex<Film>>,
    scene_integrator: IntegratorType,
    sampler: SamplerType,

    // Output
    tone_map_type: ToneMapType,
    tone_map_film: ToneMapFilm,
    ray_visualization: RayVisualization,
    bvh_visualization: BvhVisualization,

    // Scene
    load_settings: SceneLoadSettings,
    scene: Arc<Scene>,
    camera_params: CameraParameters,
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

        let ray_visualization = expect!(
            RayVisualization::new(&display),
            "Failed to create ray visualization render pass"
        );
        let bvh_visualization = expect!(
            BvhVisualization::new(&display),
            "Failed to create BVH visualization render pass"
        );

        let mut load_settings = settings.load_settings.unwrap_or_default();

        // Init with cornell here so scene is loaded on first frame and ui gets load time through the normal logic
        let (scene, camera_params, scene_film_settings, _) = match try_load_scene(&load_settings) {
            Ok(result) => result,
            Err(why) => {
                yuki_error!("Scene loading failed: {}", why);
                Scene::cornell()
            }
        };
        load_settings.path.clear();

        Window {
            event_loop,
            display,
            ui,
            tone_map_film,
            ray_visualization,
            bvh_visualization,
            film_settings: settings.film_settings.unwrap_or(scene_film_settings),
            render_settings: settings.render_settings.unwrap_or_default(),
            scene_integrator: settings.scene_integrator.unwrap_or_default(),
            sampler: settings.sampler.unwrap_or_default(),
            film,
            scene,
            tone_map_type: settings.tone_map.unwrap_or_default(),
            load_settings,
            camera_params,
        }
    }

    pub fn main_loop(self) {
        let Window {
            mut event_loop,
            display,
            mut ui,
            mut tone_map_film,
            mut ray_visualization,
            mut bvh_visualization,
            mut film_settings,
            mut render_settings,
            mut scene_integrator,
            mut sampler,
            mut film,
            mut scene,
            mut tone_map_type,
            mut load_settings,
            mut camera_params,
        } = self;

        let mut last_frame = Instant::now();

        let mut window_size = display.gl_window().window().inner_size();
        let mut render_triggered = false;
        let mut any_item_active = false;
        let mut ui_hovered = false;
        // Boxed so we can drop this at will to kill the background threads
        let mut renderer = Renderer::new();
        let mut status_messages: Option<Vec<String>> = None;
        let mut cursor_state = CursorState::default();
        let mut mouse_gesture: Option<MouseGesture> = None;
        let mut camera_offset: Option<CameraOffset> = None;
        let mut last_render_start = Instant::now();
        let mut bvh_visualization_level = -1i32;
        let mut quit = false;

        while !quit {
            superluminal_perf::begin_event("Main loop");

            let gl_window = display.gl_window();
            let window = gl_window.window();

            superluminal_perf::begin_event("Event loop");

            event_loop.run_return(|event, _, control_flow| {
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
                        // Ran out of events so let's jump back out
                        *control_flow = ControlFlow::Exit;
                    }
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::CloseRequested => {
                            yuki_trace!("main_loop: CloseRequsted");
                            quit = true;
                        }
                        WindowEvent::Resized(size) => {
                            yuki_trace!("main_loop: Resized");
                            window_size = size;
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
                                        quit = true;
                                    }
                                    VirtualKeyCode::Return => render_triggered = true,
                                    _ => {}
                                }
                            }
                        }
                        WindowEvent::ModifiersChanged(state) => {
                            cursor_state.state = state;
                        }
                        WindowEvent::CursorEntered { .. } => cursor_state.inside = true,
                        WindowEvent::CursorLeft { .. } => cursor_state.inside = false,
                        WindowEvent::CursorMoved { position, .. } => {
                            cursor_state.position = Vec2::new(position.x, position.y);
                            if let Some(gesture) = &mut mouse_gesture {
                                gesture.current_position = cursor_state.position;
                            }
                        }
                        WindowEvent::MouseWheel { delta, .. } => {
                            handle_scroll_event(delta, camera_params, &mut camera_offset);
                        }
                        WindowEvent::MouseInput { state, button, .. } => {
                            if cursor_state.inside && !any_item_active && !ui_hovered {
                                // We only want to handle input if we're not on top of interacting with imgui

                                // Ctrl+LClick fires debug ray on pixel
                                if cursor_state.state.ctrl()
                                    && button == MouseButton::Left
                                    && state == ElementState::Pressed
                                {
                                    if let Some(rays) = launch_debug_ray(
                                        &cursor_state,
                                        &display,
                                        &film,
                                        film_settings,
                                        &scene,
                                        camera_params,
                                        scene_integrator,
                                        sampler,
                                    ) {
                                        if let Err(why) =
                                            ray_visualization.set_rays(&display, &rays)
                                        {
                                            yuki_error!(
                                                "Setting rays to ray visualization failed: {:?}",
                                                why
                                            );
                                        };
                                    }
                                }

                                if mouse_gesture.is_none()
                                    && (button == MouseButton::Middle
                                        || (button == MouseButton::Left
                                            && cursor_state.state.alt()))
                                    && state == ElementState::Pressed
                                {
                                    if cursor_state.state.shift() {
                                        mouse_gesture = Some(MouseGesture {
                                            start_position: cursor_state.position,
                                            current_position: cursor_state.position,
                                            gesture: MouseGestureType::TrackPlane,
                                        });
                                    } else {
                                        mouse_gesture = Some(MouseGesture {
                                            start_position: cursor_state.position,
                                            current_position: cursor_state.position,
                                            gesture: MouseGestureType::TrackBall,
                                        });
                                    }
                                }
                            }

                            if mouse_gesture.is_some() && state == ElementState::Released {
                                mouse_gesture = None;
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            });

            superluminal_perf::end_event(); // Event loop

            if load_settings.path.exists() {
                renderer.kill();
                match try_load_scene(&load_settings) {
                    Ok((new_scene, new_camera_params, new_film_settings, total_secs)) => {
                        scene = new_scene;
                        camera_params = new_camera_params;
                        film_settings = new_film_settings;
                        ray_visualization.clear_rays();
                        bvh_visualization.clear_bounds();
                        status_messages = Some(vec![format!("Scene loaded in {:.2}s", total_secs)]);
                    }
                    Err(why) => {
                        yuki_error!("Scene loading failed: {}", why);
                        status_messages = Some(vec!["Scene loading failed".into()]);
                    }
                }
                load_settings.path.clear();
            }

            superluminal_perf::begin_event("UI");

            expect!(
                ui.platform.prepare_frame(ui.context.io_mut(), window),
                "Failed to prepare imgui gl frame"
            );
            let frame_ui = ui.context.frame();

            let ui_state = generate_ui(
                frame_ui,
                window,
                &mut film_settings,
                &mut sampler,
                &mut camera_params,
                &mut scene_integrator,
                &mut tone_map_type,
                &mut load_settings,
                &mut render_settings,
                if bvh_visualization.bounds_set() {
                    Some(&mut bvh_visualization_level)
                } else {
                    None
                },
                &scene,
                renderer.is_active(),
                &status_messages,
            );
            render_triggered |= ui_state.render_triggered;
            any_item_active = ui_state.any_item_active;
            ui_hovered = ui_state.ui_hovered;

            if ui_state.recompute_bvh_vis {
                if let Err(why) = bvh_visualization
                    .set_bounds(&display, &scene.bvh.node_bounds(bvh_visualization_level))
                {
                    yuki_error!("Setting bounds to BVH visualization failed: {:?}", why);
                };
            }

            if ui_state.clear_bvh_vis {
                bvh_visualization.clear_bounds();
            }

            render_triggered |= handle_mouse_gestures(
                window_size,
                &mut camera_params,
                &mut mouse_gesture,
                &mut camera_offset,
            );

            superluminal_perf::end_event(); // UI

            if ui_state.save_settings {
                let settings = InitialSettings {
                    film_settings: Some(film_settings),
                    sampler: Some(sampler),
                    scene_integrator: Some(scene_integrator),
                    tone_map: Some(tone_map_type),
                    load_settings: Some(SceneLoadSettings {
                        path: scene.load_settings.path.clone(),
                        max_shapes_in_node: load_settings.max_shapes_in_node,
                        split_method: load_settings.split_method,
                    }),
                    render_settings: Some(render_settings),
                };

                match File::create("settings.yaml") {
                    Ok(file) => {
                        let writer = BufWriter::new(file);
                        if let Err(why) = serde_yaml::to_writer(writer, &settings) {
                            yuki_error!("Failed to serialize settings: {}", why);
                        }
                    }
                    Err(why) => {
                        yuki_error!("Failed to create settings file: {}", why);
                    }
                }
            }

            let active_camera_params = camera_offset.as_ref().map_or(camera_params, |offset| {
                render_triggered = true; // TODO: Delta between current mouse positions to skip new render ~stationary mouse
                offset.apply(camera_params)
            });

            if render_triggered {
                superluminal_perf::begin_event("Render triggered");

                yuki_debug!("main_loop: Render triggered");
                // Make sure film matches settings
                // This leaves the previous film hanging until all threads have dropped it
                film = film_or_new(&film, film_settings);
                last_render_start = Instant::now();
                renderer.launch(
                    Arc::clone(&scene),
                    active_camera_params,
                    Arc::clone(&film),
                    sampler,
                    scene_integrator,
                    film_settings,
                    render_settings,
                );
                status_messages = Some(vec!["Render started".to_string()]);
                render_triggered = false;

                superluminal_perf::end_event(); // Render triggered
            } else {
                yuki_trace!("main_loop: Render job tracked");

                if let Some(status) = renderer.check_status() {
                    status_messages = Some(render_status_messages(&status, last_render_start));
                }
            }

            let draw_start = Instant::now();
            superluminal_perf::begin_event("Draw");

            let mut render_target = display.draw();
            render_target.clear_color_srgb(0.0, 0.0, 0.0, 0.0);

            superluminal_perf::begin_event("Draw::Tone map");

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

            superluminal_perf::end_event(); // Tone map

            superluminal_perf::begin_event("Draw::Visualizations");

            expect!(
                ray_visualization.draw(
                    scene.bvh.bounds(),
                    active_camera_params,
                    film_settings,
                    &mut tone_mapped_film.as_surface(),
                ),
                "Ray visualization failed"
            );
            expect!(
                bvh_visualization.draw(
                    scene.bvh.bounds(),
                    active_camera_params,
                    film_settings,
                    &mut tone_mapped_film.as_surface(),
                ),
                "Ray visualization failed"
            );

            superluminal_perf::end_event(); // Visualizations

            superluminal_perf::begin_event("Draw::Scale output");

            ScaleOutput::draw(tone_mapped_film, &mut render_target);

            superluminal_perf::end_event(); // Scale output

            {
                superluminal_perf::begin_event("Draw::UI");

                ui.platform
                    .prepare_render(frame_ui, display.gl_window().window());
                let draw_data = ui.context.render();
                expect!(
                    ui.renderer.render(&mut render_target, draw_data),
                    "Rendering GL window failed"
                );

                superluminal_perf::end_event(); // Draw::UI
            }

            // Finish frame
            expect!(render_target.finish(), "Frame::finish() failed");

            superluminal_perf::end_event(); // Draw

            let spent_millis = draw_start.elapsed().as_secs_f32() * 1e3;
            yuki_trace!("main_loop: Draw took {:4.2}ms", spent_millis);

            // Handle after draw so we have the mapped output texture
            if let Some(output) = &ui_state.write_exr {
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
                                        .unchecked_read::<Vec<Spectrum<f32>>, Spectrum<f32>>()
                                };
                                (w, h, pixels)
                            }
                        };

                        status_messages = Some(vec![match write_exr(w, h, &pixels, &path) {
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

            superluminal_perf::end_event(); // Main loop
        }
    }
}

struct CursorState {
    inside: bool,
    position: Vec2<f64>,
    state: glutin::event::ModifiersState,
}

impl Default for CursorState {
    fn default() -> Self {
        Self {
            inside: false,
            position: Vec2::zeros(),
            state: glutin::event::ModifiersState::default(),
        }
    }
}

struct MouseGesture {
    start_position: Vec2<f64>,
    current_position: Vec2<f64>,
    gesture: MouseGestureType,
}

#[derive(Debug)]
enum MouseGestureType {
    TrackBall,
    TrackPlane,
}

struct CameraOffset {
    position: Vec3<f32>,
    target: Vec3<f32>,
    flip_up: bool,
}

impl CameraOffset {
    fn apply(&self, params: CameraParameters) -> CameraParameters {
        CameraParameters {
            position: params.position + self.position,
            target: params.target + self.target,
            up: if self.flip_up { -params.up } else { params.up },
            fov: params.fov,
        }
    }
}

impl Default for CameraOffset {
    fn default() -> Self {
        Self {
            position: Vec3::zeros(),
            target: Vec3::zeros(),
            flip_up: false,
        }
    }
}

fn handle_mouse_gestures(
    window_size: glutin::dpi::PhysicalSize<u32>,
    camera_params: &mut CameraParameters,
    mouse_gesture: &mut Option<MouseGesture>,
    camera_offset: &mut Option<CameraOffset>,
) -> bool {
    match &mouse_gesture {
        Some(MouseGesture {
            start_position,
            current_position,
            gesture,
        }) => {
            match gesture {
                MouseGestureType::TrackBall => {
                    // Adapted from Max Liani
                    // https://maxliani.wordpress.com/2021/06/08/offline-to-realtime-camera-manipulation/
                    let drag_scale = 1.0 / 400.0;
                    let drag = (*current_position - *start_position) * drag_scale;

                    let from_target = camera_params.position - camera_params.target;

                    let horizontal_rotated_from_target =
                        &rotation(drag.x as f32, camera_params.up) * from_target;

                    let right = horizontal_rotated_from_target
                        .cross(camera_params.up)
                        .normalized();

                    let new_from_target =
                        &rotation(drag.y as f32, right) * horizontal_rotated_from_target;
                    let flip_up = right.dot(new_from_target.cross(camera_params.up)) < 0.0;

                    *camera_offset = Some(CameraOffset {
                        position: new_from_target - from_target,
                        flip_up,
                        ..CameraOffset::default()
                    });

                    true
                }
                MouseGestureType::TrackPlane => {
                    // Adapted from Max Liani
                    // https://maxliani.wordpress.com/2021/06/08/offline-to-realtime-camera-manipulation/
                    let from_target = camera_params.position - camera_params.target;
                    let dist_target = from_target.len();

                    // TODO: Adjust for aspect ratio difference between film and window
                    let drag_scale = {
                        match camera_params.fov {
                            FoV::X(angle) => {
                                let tan_half_fov = (angle.to_radians() * 0.5).tan();
                                dist_target * tan_half_fov / ((window_size.width as f32) * 0.5)
                            }
                            FoV::Y(angle) => {
                                let tan_half_fov = (angle.to_radians() * 0.5).tan();
                                dist_target * tan_half_fov / ((window_size.height as f32) * 0.5)
                            }
                        }
                    };
                    let drag = (*current_position - *start_position) * (drag_scale as f64);

                    let right = from_target.cross(camera_params.up).normalized();
                    let cam_up = right.cross(from_target).normalized();

                    let offset = -right * (drag.x as f32) + cam_up * (drag.y as f32);

                    *camera_offset = Some(CameraOffset {
                        position: offset,
                        target: offset,
                        ..CameraOffset::default()
                    });

                    true
                }
            }
        }
        None => {
            if camera_offset.is_some() {
                let offset = camera_offset.take();
                *camera_params = offset.unwrap().apply(*camera_params);

                true
            } else {
                false
            }
        }
    }
}

fn handle_scroll_event(
    delta: MouseScrollDelta,
    camera_params: CameraParameters,
    camera_offset: &mut Option<CameraOffset>,
) {
    if camera_offset.is_none() {
        let to_target = camera_params.target - camera_params.position;
        let dist_target = to_target.len();
        let fwd = to_target / dist_target;

        let scroll_scale = dist_target * 0.1;
        let scroll = match delta {
            MouseScrollDelta::LineDelta(_, y) => y,
            MouseScrollDelta::PixelDelta(delta) => delta.y as f32,
        };

        let offset = CameraOffset {
            position: fwd * scroll * scroll_scale,
            ..CameraOffset::default()
        };

        if relative_ne!(
            offset.apply(camera_params).position,
            camera_params.target,
            max_relative = 0.01
        ) {
            *camera_offset = Some(offset);
        }
    }
}

impl glium::texture::Texture2dDataSink<Spectrum<f32>> for Vec<Spectrum<f32>> {
    fn from_raw(data: Cow<'_, [Spectrum<f32>]>, _: u32, _: u32) -> Self {
        data.into()
    }
}

unsafe impl glium::texture::PixelValue for Spectrum<f32> {
    fn get_format() -> glium::texture::ClientFormat {
        glium::texture::ClientFormat::F32F32F32
    }
}

#[must_use]
fn launch_debug_ray(
    cursor_state: &CursorState,
    display: &glium::Display,
    film: &Arc<Mutex<Film>>,
    film_settings: FilmSettings,
    scene: &Arc<Scene>,
    camera_params: CameraParameters,
    scene_integrator: IntegratorType,
    sampler: SamplerType,
) -> Option<Vec<IntegratorRay>> {
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

    let collected_rays = if film_px.min_comp() >= 0.0
        && film_px.x < (film_w as f64)
        && film_px.y < (film_h as f64)
    {
        #[allow(clippy::cast_sign_loss)] // We check above
        let film_px = Vec2::new(film_px.x as u16, film_px.y as u16);

        yuki_info!(
            "main_loop: Launching debug ray at film px ({},{})",
            film_px.x,
            film_px.y
        );

        let camera = Camera::new(camera_params, film_settings);

        {
            let p_film = Point2::new(film_px.x as f32, film_px.y as f32);

            let integrator = scene_integrator.instantiate();
            let mut sampler: Box<dyn Sampler> = sampler
                .instantiate(
                    1 + scene_integrator.n_sampled_dimensions(), // Camera sample and whatever the sampler needs
                )
                .as_ref()
                .clone(0); // The interface is a bit clunky outside the renderer
            sampler.start_pixel();
            sampler.start_sample();

            let ray = camera.ray(&CameraSample {
                p_film: p_film + sampler.get_2d(),
            });

            let mut alloc = LinearAllocator::new(1024 * 256);
            let scratch = ScopedScratch::new(&mut alloc);

            let result = integrator.li(&scratch, ray, scene, 0, &mut sampler, true);
            Some(result.rays)
        }
    } else {
        yuki_info!("main_loop: Window px is outside the film");
        None
    };

    collected_rays
}

fn render_status_messages(status: &RenderStatus, render_start: Instant) -> Vec<String> {
    let elapsed_s = render_start.elapsed().as_secs_f32();

    match *status {
        RenderStatus::Finished { ray_count } => {
            vec![
                format!("Render finished in {:.2}s", elapsed_s),
                format!("{:.2} Mrays/s", ((ray_count as f32) / elapsed_s) * 1e-6),
            ]
        }
        RenderStatus::Progress {
            active_threads,
            tiles_done,
            tiles_total,
            approx_remaining_s,
            current_rays_per_s,
        } => {
            vec![
                format!("Render threads running: {}", active_threads),
                format!(
                    "{:.1}s elapsed, ~{:.0}s remaining",
                    elapsed_s, approx_remaining_s
                ),
                format!("{}/{} tiles", tiles_done, tiles_total),
                format!("{:.2} Mrays/s", current_rays_per_s * 1e-6),
            ]
        }
    }
}
