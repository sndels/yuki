use allocators::{LinearAllocator, ScopedScratch};
use approx::relative_ne;
use glium::glutin;
use glium::{
    glutin::{
        dpi::PhysicalSize,
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
    ui::{generate_ui, UIState, WriteEXR, UI},
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
    last_limit_timestamp: Option<Instant>,

    // Rendering
    film_settings: FilmSettings,
    render_settings: RenderSettings,
    film: Arc<Mutex<Film>>,
    scene_integrator: IntegratorType,
    sampler: SamplerType,
    renderer: Renderer,

    // Output
    tone_map_type: ToneMapType,
    tone_map_film: ToneMapFilm,
    output_scaler: ScaleOutput,
    ray_visualization: RayVisualization,
    bvh_visualization: BvhVisualization,

    // Scene
    load_settings: SceneLoadSettings,
    scene: Arc<Scene>,
    camera_params: CameraParameters,

    // Random main loop state
    quit: bool,
    last_frame: Instant,
    render_triggered: bool,
    launch_debug_ray: bool,
    any_item_active: bool,
    ui_hovered: bool,
    status_messages: Option<Vec<String>>,
    cursor_state: CursorState,
    mouse_gesture: Option<MouseGesture>,
    camera_offset: Option<CameraOffset>,
    last_render_start: Instant,
    bvh_visualization_level: i32,
    render_launch_timer: Instant,
    rendered_camera_offset: Option<CameraOffset>,
}

impl Window {
    pub fn new(title: &str, resolution: (u16, u16), settings: InitialSettings) -> Window {
        // Create window and gl context
        let event_loop = EventLoop::new();
        let (display, with_vsync) = {
            let window_builder = WindowBuilder::new()
                .with_title(title.to_owned())
                .with_inner_size(PhysicalSize::new(resolution.0 as f64, resolution.1 as f64));

            // No alpha and linear output as alpha and srgb seem to misbehave on wayland.
            // Double buffer vsync to neatly limit update rate to a sane number based on the screen
            let context_builder = glutin::ContextBuilder::new()
                .with_vsync(true)
                .with_pixel_format(24, 0)
                .with_srgb(false)
                .with_double_buffer(Some(true));
            if let Ok(display) =
                glium::Display::new(window_builder.clone(), context_builder, &event_loop)
            {
                (display, true)
            } else {
                // Fallback without vsync in case it's not supported
                let context_builder = glutin::ContextBuilder::new()
                    .with_pixel_format(24, 0)
                    .with_srgb(false);
                (
                    expect!(
                        glium::Display::new(window_builder, context_builder, &event_loop),
                        "Failed to initialize glium display"
                    ),
                    false,
                )
            }
        };

        // Film
        let film = Arc::new(Mutex::new(Film::default()));

        let tone_map_film = expect!(
            ToneMapFilm::new(&display),
            "Failed to create tone map render pass"
        );

        let output_scaler = expect!(
            ScaleOutput::new(&display),
            "Failed to create output scaler pass"
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
            last_limit_timestamp: if with_vsync {
                None
            } else {
                Some(Instant::now())
            },
            tone_map_film,
            output_scaler,
            ray_visualization,
            bvh_visualization,
            film_settings: settings.film_settings.unwrap_or(scene_film_settings),
            render_settings: settings.render_settings.unwrap_or_default(),
            scene_integrator: settings.scene_integrator.unwrap_or_default(),
            sampler: settings.sampler.unwrap_or_default(),
            renderer: Renderer::new(),
            film,
            scene,
            tone_map_type: settings.tone_map.unwrap_or_default(),
            load_settings,
            camera_params,
            quit: false,
            last_frame: Instant::now(),
            render_triggered: false,
            launch_debug_ray: false,
            any_item_active: false,
            ui_hovered: false,
            status_messages: None,
            cursor_state: CursorState::default(),
            mouse_gesture: None,
            camera_offset: None,
            last_render_start: Instant::now(),
            bvh_visualization_level: -1i32,
            render_launch_timer: Instant::now(),
            rendered_camera_offset: None,
        }
    }

    pub fn main_loop(mut self) {
        // Don't store in self as context.frame() locks self behind a mutable ref
        // until imgui is rendered
        let mut ui = UI::new(&self.display);

        while !self.quit {
            superluminal_perf::begin_event("Main loop");

            self.handle_events(&mut ui);

            if self.load_settings.path.exists() {
                self.load_scene();
            }

            superluminal_perf::begin_event("UI");

            expect!(
                ui.platform
                    .prepare_frame(ui.context.io_mut(), self.display.gl_window().window()),
                "Failed to prepare imgui gl frame"
            );
            let frame_ui = ui.context.frame();

            let ui_state = self.handle_ui(frame_ui);

            superluminal_perf::end_event(); // UI

            if ui_state.save_settings {
                self.save_settings();
            }

            self.handle_debug_ray();

            let active_camera_params = self.handle_camera_movement();

            self.handle_render(active_camera_params);

            let draw_start = Instant::now();
            superluminal_perf::begin_event("Draw");

            let mut render_target = self.display.draw();
            render_target.clear_color_srgb(0.0, 0.0, 0.0, 1.0);

            let tone_mapped_film = draw_tone_mapped(
                &self.display,
                &self.film,
                &mut self.tone_map_type,
                &mut self.tone_map_film,
            );

            draw_visualizations(
                &tone_mapped_film,
                &self.ray_visualization,
                &self.scene,
                active_camera_params,
                self.film_settings,
                &self.bvh_visualization,
            );

            scale_output(&self.output_scaler, tone_mapped_film, &mut render_target);

            // Can't split in a fn as frame_ui is blocking context-borrows behind a mutable ref
            {
                superluminal_perf::begin_event("Draw::UI");

                ui.platform
                    .prepare_render(frame_ui, self.display.gl_window().window());
                let draw_data = ui.context.render();
                expect!(
                    ui.renderer.render(&mut render_target, draw_data),
                    "Rendering GL window failed"
                );

                superluminal_perf::end_event(); // Draw::UI
            }

            expect!(render_target.finish(), "Frame::finish() failed");

            superluminal_perf::end_event(); // Draw

            let spent_millis = draw_start.elapsed().as_secs_f32() * 1e3;
            yuki_trace!("main_loop: Draw took {:4.2}ms", spent_millis);

            // Handle after draw so we have the mapped output texture
            handle_exr_write(
                ui_state,
                tone_mapped_film,
                &self.scene,
                &self.film,
                &mut self.status_messages,
            );

            // Limit update rate manually to something sane if we don't have double buffered vsync
            if let Some(last_instant) = self.last_limit_timestamp {
                superluminal_perf::begin_event("Limit framerate");
                loop {
                    if (Instant::now() - last_instant).as_millis() > 16 {
                        break;
                    }
                }
                self.last_limit_timestamp = Some(Instant::now());
                superluminal_perf::end_event(); // Limit framerate
            }

            superluminal_perf::end_event(); // Main loop
        }
    }

    fn handle_events(&mut self, ui: &mut UI) {
        superluminal_perf::begin_event("Event loop");

        // Every field mutated by the event loop as the borrow checker can't follow otherwise
        let last_frame = &mut self.last_frame;
        let quit = &mut self.quit;
        let render_triggered = &mut self.render_triggered;
        let cursor_state = &mut self.cursor_state;
        let mouse_gesture = &mut self.mouse_gesture;
        let camera_offset = &mut self.camera_offset;
        let launch_debug_ray = &mut self.launch_debug_ray;

        self.event_loop.run_return(|event, _, control_flow| {
            let gl_window = self.display.gl_window();
            let window = gl_window.window();

            ui.handle_event(window, &event);
            match event {
                Event::NewEvents(_) => {
                    yuki_trace!("main_loop: NewEvents");
                    let now = Instant::now();
                    ui.update_delta_time(now - *last_frame);
                    *last_frame = now;
                }
                Event::MainEventsCleared => {
                    yuki_trace!("main_loop: MainEventsCleared");
                    // Ran out of events so let's jump back out
                    *control_flow = ControlFlow::Exit;
                }
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        yuki_trace!("main_loop: CloseRequsted");
                        *quit = true;
                    }
                    WindowEvent::Resized(size) => {
                        yuki_trace!("main_loop: Resized");
                        gl_window.resize(size);
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
                        if !self.any_item_active {
                            // We only want to handle keypresses if we're not interacting with imgui
                            match key {
                                VirtualKeyCode::Escape => {
                                    *quit = true;
                                }
                                VirtualKeyCode::Return => *render_triggered = true,
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
                        if let Some(gesture) = mouse_gesture {
                            gesture.current_position = cursor_state.position;
                        }
                    }
                    WindowEvent::MouseWheel { delta, .. } => {
                        handle_scroll_event(delta, self.camera_params, camera_offset);
                    }
                    WindowEvent::MouseInput { state, button, .. } => {
                        handle_mouse_input(
                            state,
                            button,
                            self.any_item_active,
                            self.ui_hovered,
                            cursor_state,
                            mouse_gesture,
                            launch_debug_ray,
                        );
                    }
                    _ => {}
                },
                _ => {}
            }
        });

        superluminal_perf::end_event(); // Event loop
    }

    fn handle_ui(&mut self, frame_ui: &mut imgui::Ui) -> UIState {
        let gl_window = self.display.gl_window();
        let window = gl_window.window();

        let ui_state = generate_ui(
            frame_ui,
            window,
            &mut self.film_settings,
            &mut self.sampler,
            &mut self.camera_params,
            &mut self.scene_integrator,
            &mut self.tone_map_type,
            &mut self.load_settings,
            &mut self.render_settings,
            if self.bvh_visualization.bounds_set() {
                Some(&mut self.bvh_visualization_level)
            } else {
                None
            },
            &self.scene,
            self.renderer.is_active(),
            &self.status_messages,
        );
        self.render_triggered |= ui_state.render_triggered;
        self.any_item_active = ui_state.any_item_active;
        self.ui_hovered = ui_state.ui_hovered;

        if ui_state.render_killed {
            self.renderer.kill();
        }

        if ui_state.recompute_bvh_vis {
            if let Err(why) = self.bvh_visualization.set_bounds(
                &self.display,
                &self.scene.bvh.node_bounds(self.bvh_visualization_level),
            ) {
                yuki_error!("Setting bounds to BVH visualization failed: {:?}", why);
            };
        }

        if ui_state.clear_bvh_vis {
            self.bvh_visualization.clear_bounds();
        }

        self.render_triggered |= handle_mouse_gestures(
            self.display.gl_window().window().inner_size(),
            &mut self.camera_params,
            &mut self.mouse_gesture,
            &mut self.camera_offset,
        );

        ui_state
    }

    fn load_scene(&mut self) {
        self.renderer.kill();
        match try_load_scene(&self.load_settings) {
            Ok((new_scene, new_camera_params, new_film_settings, total_secs)) => {
                self.scene = new_scene;
                self.camera_params = new_camera_params;
                self.film_settings = new_film_settings;
                self.ray_visualization.clear_rays();
                self.bvh_visualization.clear_bounds();
                self.status_messages = Some(vec![format!("Scene loaded in {:.2}s", total_secs)]);
            }
            Err(why) => {
                yuki_error!("Scene loading failed: {}", why);
                self.status_messages = Some(vec!["Scene loading failed".into()]);
            }
        }
        self.load_settings.path.clear();
    }

    fn save_settings(&self) {
        let settings = InitialSettings {
            film_settings: Some(self.film_settings),
            sampler: Some(self.sampler),
            scene_integrator: Some(self.scene_integrator),
            tone_map: Some(self.tone_map_type),
            load_settings: Some(SceneLoadSettings {
                path: self.scene.load_settings.path.clone(),
                max_shapes_in_node: self.load_settings.max_shapes_in_node,
                split_method: self.load_settings.split_method,
            }),
            render_settings: Some(self.render_settings),
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

    fn handle_camera_movement(&mut self) -> CameraParameters {
        self.camera_offset
            .as_ref()
            .map_or(self.camera_params, |offset| {
                let changed = if let Some(old) = self.rendered_camera_offset {
                    old.is_different(offset)
                } else {
                    true
                };

                if changed {
                    self.rendered_camera_offset = Some(*offset);
                    self.render_triggered = true;
                }

                offset.apply(self.camera_params)
            })
    }

    fn handle_render(&mut self, active_camera_params: CameraParameters) {
        if self.render_triggered {
            // Make sure we relaunch the render at full res after a mouse gesture ends
            let res_changed = self.film.lock().unwrap().res() != self.film_settings.res;
            if (res_changed && self.mouse_gesture.is_none())
                || self.render_launch_timer.elapsed().as_millis() > 32
            {
                self.trigger_render(active_camera_params);
            } else {
                self.render_triggered = false;
            }
        } else {
            yuki_trace!("main_loop: Render job tracked");

            if let Some(status) = self.renderer.check_status() {
                self.status_messages =
                    Some(render_status_messages(&status, self.last_render_start));
            }
        }
    }

    fn trigger_render(&mut self, active_camera_params: CameraParameters) {
        superluminal_perf::begin_event("Render triggered");

        yuki_debug!("main_loop: Render triggered");
        let force_single_sample = self.mouse_gesture.is_some();

        // Modify the settings before film_or_new to get proper film
        let mut film_settings = self.film_settings;
        if force_single_sample {
            film_settings.accumulate = false;
            film_settings.clear = false;
            film_settings.sixteenth_res = true;
        } else {
            // Don't override if selected from the ui
            film_settings.sixteenth_res |= false;
            film_settings.clear = true;
        }
        if film_settings.sixteenth_res {
            film_settings.res /= 4;
        }

        // Make sure film matches settings
        // This leaves the previous film hanging until all threads have dropped it
        self.film = film_or_new(&self.film, film_settings);
        self.last_render_start = Instant::now();

        self.renderer.launch(
            Arc::clone(&self.scene),
            active_camera_params,
            Arc::clone(&self.film),
            self.sampler,
            self.scene_integrator,
            film_settings,
            self.render_settings,
            force_single_sample,
        );
        self.status_messages = Some(vec!["Render started".to_string()]);
        self.render_triggered = false;
        self.render_launch_timer = Instant::now();

        superluminal_perf::end_event(); // Render triggered
    }

    fn handle_debug_ray(&mut self) {
        if self.launch_debug_ray {
            if let Some(rays) = launch_debug_ray(
                &self.cursor_state,
                &self.display,
                &self.film,
                self.film_settings,
                &self.scene,
                self.camera_params,
                self.scene_integrator,
                self.sampler,
            ) {
                if let Err(why) = self.ray_visualization.set_rays(&self.display, &rays) {
                    yuki_error!("Setting rays to ray visualization failed: {:?}", why);
                };
            }

            self.launch_debug_ray = false;
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

#[derive(Clone, Copy)]
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

    fn is_different(&self, other: &CameraOffset) -> bool {
        relative_ne!(self.position, other.position)
            || relative_ne!(self.target, other.target)
            || self.flip_up != other.flip_up
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

                    // We detect later if this is hovering in the same place as last time
                    false
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

                    // We detect later if this is hovering in the same place as last time
                    false
                }
            }
        }
        None => {
            if let Some(offset) = camera_offset.take() {
                *camera_params = offset.apply(*camera_params);
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
            let mut sampler: Box<dyn Sampler> = sampler.instantiate(false).as_ref().clone(); // The interface is a bit clunky outside the renderer

            let ray = camera.ray(&CameraSample {
                p_film: p_film + sampler.get_2d(),
            });

            let mut alloc = LinearAllocator::new(1024 * 256);
            let scratch = ScopedScratch::new(&mut alloc);

            let mut rays = Vec::new();
            integrator.li_debug(&scratch, ray, scene, 0, &mut sampler, &mut rays);
            Some(rays)
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

/// Dumps either the tonemapped film or the raw pixels to EXR
fn dump_exr(
    path: std::path::PathBuf,
    output_type: &WriteEXR,
    tone_mapped_film: &glium::Texture2d,
    film: Arc<Mutex<Film>>,
) -> Vec<String> {
    let (w, h, pixels) = match output_type {
        WriteEXR::Raw => {
            yuki_trace!("draw: Waiting for lock on film");
            let film = film.lock().unwrap();
            yuki_trace!("draw: Acquired film");

            let film_res = film.res();
            let film_x = film_res.x as usize;
            let film_y = film_res.y as usize;
            let mut pixels = film.pixels().clone();

            // Need to average samples if there are multiple per tile
            if let Some(samples) = film.samples() {
                let tile_dim = film.tile_dim().unwrap() as usize;
                let x_tiles = (film_x.saturating_sub(1) / tile_dim) + 1;
                for j in 0..film_y {
                    let row_start_px = j * film_x;
                    let row_start_tile = (j / tile_dim) * x_tiles;
                    for i in 0..film_x {
                        let tile_i = i / tile_dim;
                        // Might be zero samples, use 1 instead assuming zeroed film
                        pixels[row_start_px + i] /=
                            (samples[row_start_tile + tile_i] as f32).max(1.0);
                    }
                }
            }

            yuki_trace!("draw: Releasing film");
            (film_res.x as usize, film_res.y as usize, pixels)
        }

        WriteEXR::Mapped => {
            let w = tone_mapped_film.width() as usize;
            let h = tone_mapped_film.height() as usize;
            // TODO: This will explode if mapped texture format is not f32f32f32
            let pixels =
                unsafe { tone_mapped_film.unchecked_read::<Vec<Spectrum<f32>>, Spectrum<f32>>() };
            (w, h, pixels)
        }
    };

    vec![match write_exr(w, h, &pixels, &path) {
        Ok(_) => "EXR written".into(),
        Err(why) => {
            yuki_error!("{}", why);
            "Error writing EXR".into()
        }
    }]
}

fn draw_tone_mapped<'a>(
    display: &glium::Display,
    film: &Arc<Mutex<Film>>,
    tone_map_type: &mut ToneMapType,
    tone_map_film: &'a mut ToneMapFilm,
) -> &'a glium::Texture2d {
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
                find_min_max(film, *channel),
                "Failed to find film min, max"
            ));
        }
    }

    let tone_mapped_film = expect!(
        tone_map_film.draw(display, film, tone_map_type),
        "Film tone map pass failed"
    );

    superluminal_perf::end_event(); // Tone map

    tone_mapped_film
}

fn draw_visualizations(
    tone_mapped_film: &glium::Texture2d,
    ray_visualization: &RayVisualization,
    scene: &Scene,
    active_camera_params: CameraParameters,
    film_settings: FilmSettings,
    bvh_visualization: &BvhVisualization,
) {
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
}

fn scale_output(
    output_scaler: &ScaleOutput,
    tone_mapped_film: &glium::Texture2d,
    render_target: &mut glium::Frame,
) {
    superluminal_perf::begin_event("Draw::Scale output");

    expect!(
        output_scaler.draw(tone_mapped_film, render_target),
        "Output scaling failed"
    );

    superluminal_perf::end_event(); // Scale output
}

fn handle_exr_write(
    ui_state: UIState,
    tone_mapped_film: &glium::Texture2d,
    scene: &Scene,
    film: &Arc<Mutex<Film>>,
    status_messages: &mut Option<Vec<String>>,
) {
    if let Some(output_type) = &ui_state.write_exr {
        match exr_path(&scene) {
            Ok(path) => {
                *status_messages = Some(dump_exr(
                    path,
                    output_type,
                    tone_mapped_film,
                    Arc::clone(film),
                ));
            }
            Err(why) => {
                yuki_error!("{}", why);
            }
        }
    }
}

fn handle_mouse_input(
    state: ElementState,
    button: MouseButton,
    any_item_active: bool,
    ui_hovered: bool,
    cursor_state: &CursorState,
    mouse_gesture: &mut Option<MouseGesture>,
    launch_debug_ray: &mut bool,
) {
    if cursor_state.inside && !any_item_active && !ui_hovered {
        // We only want to handle input if we're not on top of interacting with imgui

        // Ctrl+LClick fires debug ray on pixel
        if cursor_state.state.ctrl()
            && button == MouseButton::Left
            && state == ElementState::Pressed
        {
            *launch_debug_ray = true;
        }

        if mouse_gesture.is_none()
            && (button == MouseButton::Middle
                || (button == MouseButton::Left && cursor_state.state.alt()))
            && state == ElementState::Pressed
        {
            if cursor_state.state.shift() {
                *mouse_gesture = Some(MouseGesture {
                    start_position: cursor_state.position,
                    current_position: cursor_state.position,
                    gesture: MouseGestureType::TrackPlane,
                });
            } else {
                *mouse_gesture = Some(MouseGesture {
                    start_position: cursor_state.position,
                    current_position: cursor_state.position,
                    gesture: MouseGestureType::TrackBall,
                });
            }
        }
    }

    if mouse_gesture.is_some() && state == ElementState::Released {
        *mouse_gesture = None;
    }
}
