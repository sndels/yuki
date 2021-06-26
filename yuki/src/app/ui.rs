// Adapted from imgui-rs glium example

use glutin::{event::Event, window::Window};
use imgui::Context;
use imgui::{im_str, FontConfig, FontSource, ImStr};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::{convert::TryFrom, path::PathBuf, sync::Arc, time::Duration};
use strum::VariantNames;
use tinyfiledialogs::open_file_dialog;

use crate::{
    camera::FoV,
    expect,
    film::FilmSettings,
    integrators::IntegratorType,
    math::Vec2,
    samplers::SamplerSettings,
    scene::{CameraOrientation, DynamicSceneParameters, Scene, SceneLoadSettings},
    yuki_error,
};

const MIN_TILE: u16 = 8;
const MIN_RES: u16 = 64;
const MAX_RES: u16 = 4096;
const RES_STEP: u16 = 2;
const TILE_STEP: u16 = 2;
const MAX_SAMPLES: u16 = 32;

pub struct UI {
    context: Context,
    platform: WinitPlatform,
    renderer: Renderer,
}

impl UI {
    pub fn new(display: &glium::Display) -> Self {
        let mut context = imgui::Context::create();
        context.set_ini_filename(None);

        let mut platform = WinitPlatform::init(&mut context);

        let hidpi_factor = platform.hidpi_factor();
        let font_size = (13.0 * hidpi_factor) as f32;
        context.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(FontConfig {
                size_pixels: font_size,
                ..FontConfig::default()
            }),
        }]);

        context.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        {
            let style = context.style_mut();
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
        }

        let renderer = expect!(
            Renderer::init(&mut context, display),
            "Failed to initialize renderer"
        );

        platform.attach_window(
            context.io_mut(),
            display.gl_window().window(),
            HiDpiMode::Rounded,
        );

        Self {
            context,
            platform,
            renderer,
        }
    }

    pub fn handle_event<'b, T: 'static>(&mut self, window: &Window, event: &Event<'b, T>) {
        self.platform
            .handle_event(self.context.io_mut(), window, event);
    }

    pub fn update_delta_time(&mut self, delta: Duration) {
        self.context.io_mut().update_delta_time(delta);
    }

    pub fn generate_frame(
        &mut self,
        window: &glutin::window::Window,
        film_settings: &mut FilmSettings,
        exposure: &mut f32,
        sampler_settings: &mut SamplerSettings,
        scene_params: &mut DynamicSceneParameters,
        scene_integrator: &mut IntegratorType,
        load_settings: &mut SceneLoadSettings,
        match_logical_cores: &mut bool,
        scene: Arc<Scene>,
        render_in_progress: bool,
        status_messages: &Option<Vec<String>>,
    ) -> FrameUI {
        expect!(
            self.platform.prepare_frame(self.context.io_mut(), window),
            "Failed to prepare imgui gl frame"
        );

        let glutin::dpi::PhysicalSize {
            width: _,
            height: window_height,
        } = window.inner_size();

        let ui = self.context.frame();
        let mut render_triggered = false;
        let mut write_exr = false;
        let mut scene_path = None;

        imgui::Window::new(im_str!("Settings"))
            .position([0.0, 0.0], imgui::Condition::Always)
            .size([370.0, window_height as f32], imgui::Condition::Always)
            .resizable(false)
            .movable(false)
            .build(&ui, || {
                render_triggered |= generate_film_settings(&ui, film_settings, exposure);
                ui.spacing();

                render_triggered |= generate_sampler_settings(&ui, sampler_settings);
                ui.spacing();

                render_triggered |= generate_scene_settings(
                    &ui,
                    &scene,
                    scene_params,
                    load_settings,
                    &mut scene_path,
                );
                ui.spacing();

                render_triggered |= generate_integrator_settings(&ui, scene_integrator);
                ui.spacing();

                ui.checkbox(im_str!("Match logical cores"), match_logical_cores);
                ui.spacing();

                render_triggered |= ui.button(im_str!("Render"), [50.0, 20.0]);
                if !render_in_progress {
                    ui.same_line(0.0);
                    write_exr |= ui.button(im_str!("Write EXR"), [75.0, 20.0]);
                }
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

        let any_item_active = ui.is_any_item_active();

        FrameUI {
            platform: &mut self.platform,
            renderer: &mut self.renderer,
            ui: Some(ui),
            render_triggered,
            write_exr,
            scene_path,
            any_item_active,
        }
    }
}

/// Kind of a closure that gets around having to store imgui::UI within UI during a frame
pub struct FrameUI<'a> {
    platform: &'a mut WinitPlatform,
    renderer: &'a mut Renderer,
    ui: Option<imgui::Ui<'a>>,
    pub render_triggered: bool,
    pub write_exr: bool,
    pub scene_path: Option<PathBuf>,
    pub any_item_active: bool,
}

impl<'a> FrameUI<'a> {
    pub fn end_frame(&mut self, display: &glium::Display, render_target: &mut glium::Frame) {
        if let Some(ui) = std::mem::replace(&mut self.ui, None) {
            self.platform
                .prepare_render(&ui, display.gl_window().window());
            expect!(
                self.renderer.render(render_target, ui.render()),
                "Rendering GL window failed"
            );
        } else {
            yuki_error!("UI::end_frame called twice on the same object!");
        }
    }
}

impl<'a> Drop for FrameUI<'a> {
    fn drop(&mut self) {
        if let Some(_) = self.ui {
            yuki_error!("FrameUI::end_frame was not called!");
        }
    }
}

/// Returns `true` if the value was changed.
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

/// Returns `true` if the value was changed.
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

/// Returns `true` if film_settings was changed.
fn generate_film_settings(
    ui: &imgui::Ui<'_>,
    film_settings: &mut FilmSettings,
    exposure: &mut f32,
) -> bool {
    let mut changed = false;

    imgui::TreeNode::new(im_str!("Film"))
        .default_open(true)
        .build(&ui, || {
            changed |= vec2_u16_picker(
                &ui,
                im_str!("Resolution"),
                &mut film_settings.res,
                MIN_RES,
                MAX_RES,
                RES_STEP as f32,
            );

            {
                let width = ui.push_item_width(118.0);
                changed |= u16_picker(
                    &ui,
                    im_str!("Tile size"),
                    &mut film_settings.tile_dim,
                    MIN_TILE,
                    MIN_RES,
                    TILE_STEP as f32,
                );
                width.pop(&ui);
            }

            {
                let width = ui.push_item_width(118.0);
                imgui::Drag::new(im_str!("Exposure"))
                    .range(0.0..=f32::MAX)
                    .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                    .speed(0.001)
                    .display_format(im_str!("%.3f"))
                    .build(&ui, exposure);
                width.pop(&ui);
            }

            if ui.checkbox(im_str!("Clear buffer"), &mut film_settings.clear) && film_settings.clear
            {
                changed = true;
            }
        });

    changed
}

/// Returns `true` if sampler_settings was changed.
fn generate_sampler_settings(ui: &imgui::Ui<'_>, sampler_settings: &mut SamplerSettings) -> bool {
    let mut changed = false;

    imgui::TreeNode::new(im_str!("Sampler"))
        .default_open(true)
        .build(&ui, || {
            // TODO: Sampler picker
            match sampler_settings {
                SamplerSettings::StratifiedSampler {
                    pixel_samples,
                    symmetric_dimensions,
                    jitter_samples,
                } => {
                    if *symmetric_dimensions {
                        let width = ui.push_item_width(118.0);
                        changed |= u16_picker(
                            &ui,
                            im_str!("Pixel extent samples"),
                            &mut pixel_samples.x,
                            1,
                            MAX_SAMPLES,
                            1.0,
                        );
                        width.pop(&ui);
                        pixel_samples.y = pixel_samples.x;
                    } else {
                        changed |= vec2_u16_picker(
                            &ui,
                            im_str!("Pixel samples"),
                            pixel_samples,
                            1,
                            MAX_SAMPLES,
                            1.0,
                        );
                    }
                    changed |= ui.checkbox(im_str!("Symmetric dimensions"), symmetric_dimensions);
                    changed |= ui.checkbox(im_str!("Jitter samples"), jitter_samples);
                    ui.text(im_str!(
                        "Samples per pixel: {}",
                        pixel_samples.x * pixel_samples.y
                    ));
                }
            }
        });

    changed
}

/// Returns `true` if camera settings were changed.
fn generate_scene_settings(
    ui: &imgui::Ui<'_>,
    scene: &Scene,
    params: &mut DynamicSceneParameters,
    load_settings: &mut SceneLoadSettings,
    scene_path: &mut Option<PathBuf>,
) -> bool {
    let mut changed = false;
    imgui::TreeNode::new(im_str!("Scene"))
        .default_open(true)
        .build(&ui, || {
            imgui::TreeNode::new(im_str!("Camera"))
                .default_open(true)
                .build(&ui, || {
                    match &mut params.cam_orientation {
                        CameraOrientation::LookAt {
                            ref mut cam_pos,
                            ref mut cam_target,
                        } => {
                            changed |= imgui::Drag::new(im_str!("Position"))
                                .speed(0.1)
                                .display_format(im_str!("%.1f"))
                                .build_array(&ui, cam_pos.array_mut());

                            changed |= imgui::Drag::new(im_str!("Target"))
                                .speed(0.1)
                                .display_format(im_str!("%.1f"))
                                .build_array(&ui, cam_target.array_mut());
                        }
                        CameraOrientation::Pose {
                            ref mut cam_pos,
                            ref mut cam_euler_deg,
                        } => {
                            changed |= imgui::Drag::new(im_str!("Position"))
                                .speed(0.1)
                                .display_format(im_str!("%.1f"))
                                .build_array(&ui, cam_pos.array_mut());

                            changed |= imgui::Drag::new(im_str!("Rotation"))
                                .speed(0.1)
                                .display_format(im_str!("%.1f"))
                                .build_array(&ui, cam_euler_deg.array_mut());
                        }
                    }

                    {
                        let width = ui.push_item_width(77.0);
                        let fov = match &mut params.cam_fov {
                            FoV::X(ref mut v) => v,
                            FoV::Y(ref mut v) => v,
                        };
                        changed |= imgui::Drag::new(im_str!("Field of View"))
                            .range(0.1..=359.9)
                            .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                            .speed(0.5)
                            .display_format(im_str!("%.1f"))
                            .build(&ui, fov);
                        width.pop(&ui);
                    }
                });

            ui.spacing();

            {
                let width = ui.push_item_width(92.0);
                u16_picker(
                    &ui,
                    im_str!("Max shapes in BVH node"),
                    &mut load_settings.max_shapes_in_node,
                    1,
                    u16::max_value(),
                    1.0,
                );
                width.pop(&ui);
            }

            ui.spacing();

            if ui.button(im_str!("Change scene"), [92.0, 20.0]) {
                let open_path = if let Some(path) = &scene.path {
                    path.to_str().unwrap()
                } else {
                    ""
                };
                *scene_path = if let Some(path) = open_file_dialog(
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
                *scene_path = scene.path.clone();
            }
        });

    changed
}

/// Returns `true` if the integrator was changed.
fn generate_integrator_settings(ui: &imgui::Ui<'_>, integrator: &mut IntegratorType) -> bool {
    let width = ui.push_item_width(140.0);

    let integrator_names = IntegratorType::VARIANTS
        .iter()
        .map(|&n| imgui::ImString::new(n))
        .collect::<Vec<imgui::ImString>>();
    // TODO: This double map is dumb. Is there a cleaner way to pass these for ComboBox?
    let im_str_integrator_names = integrator_names
        .iter()
        .map(|n| n.as_ref())
        .collect::<Vec<&imgui::ImStr>>();
    let mut current_integrator = *integrator as usize;
    let changed = imgui::ComboBox::new(im_str!("Scene integrator")).build_simple_string(
        &ui,
        &mut current_integrator,
        &im_str_integrator_names,
    );
    *integrator = IntegratorType::try_from(current_integrator).unwrap();

    width.pop(&ui);

    changed
}
