// Adapted from imgui-rs glium example

use glutin::{event::Event, window::Window};
use imgui::Context;
use imgui::{im_str, FontConfig, FontSource, ImStr};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::{
    convert::TryFrom, path::PathBuf, str::FromStr, string::ToString, sync::Arc, time::Duration,
};
use strum::VariantNames;
use tinyfiledialogs::open_file_dialog;

use super::renderpasses::{FilmicParams, HeatmapParams, ToneMapType};

use crate::{
    camera::{CameraParameters, FoV},
    expect,
    film::FilmSettings,
    integrators::{IntegratorType, PathParams, WhittedParams},
    math::{Vec2, Vec3},
    renderer::RenderSettings,
    sampling::{SamplerType, StratifiedParams, UniformParams},
    scene::{Scene, SceneLoadSettings},
    yuki_error,
};

const MIN_TILE: u16 = 8;
const MIN_RES: u16 = 64;
const MAX_RES: u16 = 4096;
const RES_STEP: u16 = 2;
const TILE_STEP: u16 = 2;
const MAX_SAMPLES: u16 = 4096;

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
        sampler: &mut SamplerType,
        camera_params: &mut CameraParameters,
        scene_integrator: &mut IntegratorType,
        tone_map_type: &mut ToneMapType,
        load_settings: &mut SceneLoadSettings,
        render_settings: &mut RenderSettings,
        scene: &Arc<Scene>,
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
        let mut write_exr = None;
        let mut save_settings = false;
        // This should be collected for all windows
        let mut ui_hovered = false;

        imgui::Window::new(im_str!("Settings"))
            .position([0.0, 0.0], imgui::Condition::Always)
            .size([370.0, window_height as f32], imgui::Condition::Always)
            .resizable(false)
            .movable(false)
            .build(&ui, || {
                ui_hovered = ui.is_window_hovered();

                render_triggered |= generate_film_settings(&ui, film_settings);
                ui.spacing();

                render_triggered |= generate_sampler_settings(&ui, sampler);
                ui.spacing();

                render_triggered |=
                    generate_scene_settings(&ui, scene, camera_params, load_settings);
                ui.spacing();

                render_triggered |= generate_integrator_settings(&ui, scene_integrator);
                ui.spacing();

                generate_tone_map_settings(&ui, tone_map_type);
                ui.spacing();

                generate_render_settings(&ui, render_settings);
                ui.spacing();

                save_settings |= ui.button(im_str!("Save settings"), [100.0, 20.0]);
                ui.spacing();

                ui.separator();
                ui.spacing();

                render_triggered |= ui.button(im_str!("Render"), [50.0, 20.0]);
                ui.spacing();

                if !render_in_progress {
                    if ui.button(im_str!("Write raw EXR"), [100.0, 20.0]) {
                        write_exr = Some(WriteEXR::Raw);
                    }
                    ui.same_line(0.0);
                    if ui.button(im_str!("Write mapped EXR"), [120.0, 20.0]) {
                        write_exr = Some(WriteEXR::Mapped);
                    }
                }
                ui.spacing();

                ui.separator();

                ui.text(im_str!("Current scene: {}", scene.name));
                ui.text(im_str!("Shape count: {}", scene.shapes.len()));
                ui.text(im_str!(
                    "Shapes in BVH node: {}",
                    (scene.load_settings.max_shapes_in_node as usize).min(scene.shapes.len())
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
            any_item_active,
            ui_hovered,
            save_settings,
        }
    }
}

pub enum WriteEXR {
    Raw,
    Mapped,
}

/// Kind of a closure that gets around having to store [`imgui::UI`] within UI during a frame
pub struct FrameUI<'a> {
    platform: &'a mut WinitPlatform,
    renderer: &'a mut Renderer,
    ui: Option<imgui::Ui<'a>>,
    pub render_triggered: bool,
    pub write_exr: Option<WriteEXR>,
    pub any_item_active: bool,
    pub ui_hovered: bool,
    pub save_settings: bool,
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
        if self.ui.is_some() {
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

    *v = u16::try_from(vi).unwrap();

    value_changed
}

/// Returns `true` if the value was changed.
fn u32_picker(ui: &imgui::Ui, label: &ImStr, v: &mut u32, min: u32, max: u32, speed: f32) -> bool {
    let mut vi = *v as i64;

    let value_changed = imgui::Drag::new(label)
        .range((min as i64)..=(max as i64))
        .flags(imgui::SliderFlags::ALWAYS_CLAMP)
        .speed(speed)
        .build(ui, &mut vi);

    *v = u32::try_from(vi).unwrap();

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

    v.x = u16::try_from(vi[0]).unwrap();
    v.y = u16::try_from(vi[1]).unwrap();

    value_changed
}

/// Returns `true` if `film_settings` was changed.
fn generate_film_settings(ui: &imgui::Ui<'_>, film_settings: &mut FilmSettings) -> bool {
    let mut changed = false;
    imgui::TreeNode::new(im_str!("Film"))
        .default_open(true)
        .build(ui, || {
            changed |= vec2_u16_picker(
                ui,
                im_str!("Resolution"),
                &mut film_settings.res,
                MIN_RES,
                MAX_RES,
                RES_STEP as f32,
            );

            {
                let width = ui.push_item_width(118.0);
                changed |= u16_picker(
                    ui,
                    im_str!("Tile size"),
                    &mut film_settings.tile_dim,
                    MIN_TILE,
                    MIN_RES,
                    TILE_STEP as f32,
                );
                width.pop(ui);
            }

            changed |= ui.checkbox(im_str!("Clear buffer"), &mut film_settings.clear)
                && film_settings.clear; // Relaunch doesn't make sense if clear is unset
        });

    changed
}

fn generate_render_settings(ui: &imgui::Ui<'_>, render_settings: &mut RenderSettings) {
    imgui::TreeNode::new(im_str!("Renderer"))
        .default_open(true)
        .build(ui, || {
            ui.checkbox(im_str!("Mark work tiles"), &mut render_settings.mark_tiles);
        });
}

/// Returns `true` if `sampler` was changed.
fn generate_sampler_settings(ui: &imgui::Ui<'_>, sampler: &mut SamplerType) -> bool {
    let mut changed = false;
    imgui::TreeNode::new(im_str!("Sampler"))
        .default_open(true)
        .build(ui, || {
            changed |= enum_combo_box(ui, im_str!("##SamplerEnum"), sampler);

            ui.indent();
            match sampler {
                SamplerType::Uniform(UniformParams { pixel_samples }) => {
                    let width = ui.push_item_width(118.0);
                    changed |= u32_picker(
                        ui,
                        im_str!("Pixel extent samples"),
                        pixel_samples,
                        1,
                        MAX_SAMPLES as u32,
                        1.0,
                    );
                    width.pop(ui);
                }
                SamplerType::Stratified(StratifiedParams {
                    pixel_samples,
                    symmetric_dimensions,
                    jitter_samples,
                }) => {
                    #[allow(clippy::cast_sign_loss)] // MAX_SAMPLES is u16
                    let max_dim = f64::from(MAX_SAMPLES).sqrt() as u16;
                    if *symmetric_dimensions {
                        let width = ui.push_item_width(118.0);
                        changed |= u16_picker(
                            ui,
                            im_str!("Pixel extent samples"),
                            &mut pixel_samples.x,
                            1,
                            max_dim,
                            1.0,
                        );
                        width.pop(ui);
                        pixel_samples.y = pixel_samples.x;
                    } else {
                        changed |= vec2_u16_picker(
                            ui,
                            im_str!("Pixel samples"),
                            pixel_samples,
                            1,
                            max_dim,
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
            ui.unindent();
        });

    changed
}

/// Returns `true` if camera settings were changed.
fn generate_scene_settings(
    ui: &imgui::Ui<'_>,
    scene: &Scene,
    camera_params: &mut CameraParameters,
    load_settings: &mut SceneLoadSettings,
) -> bool {
    let mut changed = false;
    imgui::TreeNode::new(im_str!("Scene"))
        .default_open(true)
        .build(ui, || {
            imgui::TreeNode::new(im_str!("Camera"))
                .default_open(true)
                .build(ui, || {
                    changed |= imgui::Drag::new(im_str!("Position"))
                        .speed(0.1)
                        .display_format(im_str!("%.1f"))
                        .build_array(ui, camera_params.position.array_mut());

                    changed |= imgui::Drag::new(im_str!("Target"))
                        .speed(0.1)
                        .display_format(im_str!("%.1f"))
                        .build_array(ui, camera_params.target.array_mut());

                    {
                        let width = ui.push_item_width(77.0);
                        let fov = match &mut camera_params.fov {
                            FoV::X(ref mut v) | FoV::Y(ref mut v) => v,
                        };
                        changed |= imgui::Drag::new(im_str!("Field of View"))
                            .range(0.1..=359.9)
                            .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                            .speed(0.5)
                            .display_format(im_str!("%.1f"))
                            .build(ui, fov);
                        width.pop(ui);
                    }

                    if ui.button(im_str!("Set +Y up"), [77.0, 20.0]) {
                        camera_params.up = Vec3::new(0.0, 1.0, 0.0);
                        changed = true;
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
                let open_path = &scene.load_settings.path.to_str().unwrap();
                let path = open_file_dialog(
                    "Open scene",
                    open_path,
                    Some((&["*.ply", "*.xml", "*.pbrt"], "Supported scene formats")),
                )
                .map_or_else(PathBuf::new, PathBuf::from);
                (*load_settings).path = path;
            }
            ui.same_line(0.0);
            if ui.button(im_str!("Reload scene"), [92.0, 20.0]) {
                (*load_settings).path = scene.load_settings.path.clone();
            }
        });

    changed
}

/// Returns `true` if the integrator was changed.
fn generate_integrator_settings(ui: &imgui::Ui<'_>, integrator: &mut IntegratorType) -> bool {
    let mut changed = false;
    imgui::TreeNode::new(im_str!("Integrator"))
        .default_open(true)
        .build(ui, || {
            changed |= enum_combo_box(ui, im_str!("##IntegratorEnum"), integrator);

            ui.indent();
            match integrator {
                IntegratorType::Whitted(WhittedParams { max_depth })
                | IntegratorType::Path(PathParams { max_depth }) => {
                    let width = ui.push_item_width(118.0);
                    changed |= imgui::Drag::new(im_str!("Max depth##Integrator"))
                        .range(1..=u32::MAX)
                        .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                        .build(ui, max_depth);
                    width.pop(ui);
                }
                IntegratorType::BVHIntersections
                | IntegratorType::GeometryNormals
                | IntegratorType::ShadingNormals => (),
            }
            ui.unindent();
        });

    changed
}

fn generate_tone_map_settings(ui: &imgui::Ui<'_>, params: &mut ToneMapType) {
    imgui::TreeNode::new(im_str!("Tone map"))
        .default_open(true)
        .build(ui, || {
            enum_combo_box(ui, im_str!("##ToneMapEnum"), params);
            ui.indent();
            match params {
                ToneMapType::Raw => (),
                ToneMapType::Filmic(FilmicParams { exposure }) => {
                    let width = ui.push_item_width(118.0);
                    imgui::Drag::new(im_str!("Exposure##ToneMap"))
                        .range(0.0..=f32::MAX)
                        .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                        .speed(0.001)
                        .display_format(im_str!("%.3f"))
                        .build(ui, exposure);
                    width.pop(ui);
                }
                ToneMapType::Heatmap(HeatmapParams { bounds, channel }) => {
                    let changed = enum_combo_box(ui, im_str!("Channel##Heatmap"), channel);
                    if changed {
                        *bounds = None;
                    }

                    if let Some((min, max)) = bounds {
                        let speed = ((*max - *min) / 100.0).max(0.001);
                        let width = ui.push_item_width(118.0);
                        imgui::Drag::new(im_str!("Min##Heatmap"))
                            .range(0.0..=(*max - 0.001).max(0.0))
                            .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                            .speed(speed)
                            .display_format(im_str!("%.3f"))
                            .build(ui, min);
                        ui.same_line(0.0);
                        imgui::Drag::new(im_str!("Max##Heatmap"))
                            .range((*min + 0.001)..=f32::MAX)
                            .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                            .speed(speed)
                            .display_format(im_str!("%.3f"))
                            .build(ui, max);
                        width.pop(ui);
                    }
                }
            }
            ui.unindent();
        });
}

// Generates a combo box for `value` and returns true if it changed.
fn enum_combo_box<T>(ui: &imgui::Ui<'_>, name: &ImStr, value: &mut T) -> bool
where
    T: VariantNames + ToString + FromStr,
    T::Err: std::fmt::Debug,
{
    let t_names = T::VARIANTS
        .iter()
        .map(|&n| imgui::ImString::new(n))
        .collect::<Vec<imgui::ImString>>();
    // TODO: This double map is dumb. Is there a cleaner way to pass these for ComboBox?
    let im_str_t_names = t_names
        .iter()
        .map(std::convert::AsRef::as_ref)
        .collect::<Vec<&imgui::ImStr>>();

    let mut current_t = T::VARIANTS
        .iter()
        .position(|&n| n == value.to_string())
        .unwrap();

    let changed =
        imgui::ComboBox::new(name).build_simple_string(ui, &mut current_t, &im_str_t_names);

    if changed {
        *value = T::from_str(T::VARIANTS[current_t]).unwrap();
    }

    changed
}
