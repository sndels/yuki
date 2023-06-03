// Adapted from imgui-rs glium example

use glium::glutin;
use imgui::Context;
use imgui::{FontConfig, FontSource};
use imgui_glium_renderer::Renderer;
use imgui_winit_support::{HiDpiMode, WinitPlatform};
use std::{path::PathBuf, str::FromStr, sync::Arc, time::Duration};
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
};

const MIN_TILE: u16 = 8;
const MIN_RES: u16 = 64;
const MAX_RES: u16 = 4096;
const RES_STEP: u16 = 2;
const TILE_STEP: u16 = 2;
const MAX_SAMPLES: u16 = 4096;

pub struct UI {
    pub context: Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
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

    pub fn handle_event<'b, T: 'static>(
        &mut self,
        window: &glutin::window::Window,
        event: &glutin::event::Event<'b, T>,
    ) {
        self.platform
            .handle_event(self.context.io_mut(), window, event);
    }

    pub fn update_delta_time(&mut self, delta: Duration) {
        self.context.io_mut().update_delta_time(delta);
    }
}

pub enum WriteEXR {
    Raw,
    Mapped,
}

#[allow(clippy::struct_excessive_bools)]
pub struct UIState {
    pub render_triggered: bool,
    pub render_killed: bool,
    pub write_exr: Option<WriteEXR>,
    pub any_item_active: bool,
    pub ui_hovered: bool,
    pub save_settings: bool,
    pub recompute_bvh_vis: bool,
    pub clear_bvh_vis: bool,
}

pub fn generate_ui(
    ui: &mut imgui::Ui,
    window: &glutin::window::Window,
    film_settings: &mut FilmSettings,
    sampler: &mut SamplerType,
    camera_params: &mut CameraParameters,
    scene_integrator: &mut IntegratorType,
    tone_map_type: &mut ToneMapType,
    load_settings: &mut SceneLoadSettings,
    render_settings: &mut RenderSettings,
    bvh_visualization_level: Option<&mut i32>,
    scene: &Arc<Scene>,
    render_in_progress: bool,
    status_messages: &Option<Vec<String>>,
) -> UIState {
    let glutin::dpi::PhysicalSize {
        width: _,
        height: window_height,
    } = window.inner_size();

    let mut render_triggered = false;
    let mut render_killed = false;
    let mut write_exr = None;
    let mut save_settings = false;
    let mut recompute_bvh_vis = false;
    let mut clear_bvh_vis = false;
    // This should be collected for all windows
    let mut ui_hovered = false;

    ui.window("Settings")
        .position([0.0, 0.0], imgui::Condition::Always)
        .size([370.0, window_height as f32], imgui::Condition::Always)
        .resizable(false)
        .movable(false)
        .build(|| {
            ui_hovered = ui.is_window_hovered();

            render_triggered |= generate_film_settings(ui, film_settings);
            ui.spacing();

            render_triggered |= generate_sampler_settings(ui, sampler);
            ui.spacing();

            render_triggered |= generate_scene_settings(ui, scene, camera_params, load_settings);
            ui.spacing();

            render_triggered |= generate_integrator_settings(ui, scene_integrator);
            ui.spacing();

            generate_tone_map_settings(ui, tone_map_type);
            ui.spacing();

            generate_render_settings(ui, render_settings);
            ui.spacing();

            save_settings |= ui.button("Save settings");
            ui.spacing();

            ui.separator();
            ui.spacing();

            if render_in_progress {
                render_killed |= ui.button("Kill render");
            } else {
                render_triggered |= ui.button("Render");
            }
            ui.spacing();

            if let Some(level) = bvh_visualization_level {
                let _width = ui.push_item_width(102.0);
                recompute_bvh_vis |= imgui::Drag::new("BVH level")
                    .range(-1, i32::MAX)
                    .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                    .speed(0.2)
                    .build(ui, level);
                clear_bvh_vis |= ui.button("Clear BVH visualization");
            } else {
                recompute_bvh_vis |= ui.button("Visualize BVH");
            }
            ui.spacing();

            if !render_in_progress {
                if ui.button("Write raw EXR") {
                    write_exr = Some(WriteEXR::Raw);
                }
                ui.same_line();
                if ui.button("Write mapped EXR") {
                    write_exr = Some(WriteEXR::Mapped);
                }
            }
            ui.spacing();

            ui.separator();

            ui.text(format!("Current scene: {}", scene.name));
            ui.text(format!("Shape count: {}", scene.shapes.len()));
            ui.text(format!(
                "Shapes in BVH node: {}",
                (scene.load_settings.max_shapes_in_node as usize).min(scene.shapes.len())
            ));
            ui.spacing();

            ui.separator();

            if let Some(lines) = status_messages {
                for l in lines {
                    ui.text(l);
                }
            }
        });

    let any_item_active = ui.is_any_item_active();

    UIState {
        render_triggered,
        render_killed,
        write_exr,
        any_item_active,
        ui_hovered,
        save_settings,
        recompute_bvh_vis,
        clear_bvh_vis,
    }
}

/// Returns `true` if the value was changed.
fn u16_picker(ui: &imgui::Ui, label: &str, v: &mut u16, min: u16, max: u16, speed: f32) -> bool {
    let mut vi = *v as i32;

    let value_changed = imgui::Drag::new(label)
        .range(min as i32, max as i32)
        .flags(imgui::SliderFlags::ALWAYS_CLAMP)
        .speed(speed)
        .build(ui, &mut vi);

    *v = u16::try_from(vi).unwrap();

    value_changed
}

/// Returns `true` if the value was changed.
fn u32_picker(ui: &imgui::Ui, label: &str, v: &mut u32, min: u32, max: u32, speed: f32) -> bool {
    let mut vi = *v as i64;

    let value_changed = imgui::Drag::new(label)
        .range(min as i64, max as i64)
        .flags(imgui::SliderFlags::ALWAYS_CLAMP)
        .speed(speed)
        .build(ui, &mut vi);

    *v = u32::try_from(vi).unwrap();

    value_changed
}

/// Returns `true` if the value was changed.
fn vec2_u16_picker(
    ui: &imgui::Ui,
    label: &str,
    v: &mut Vec2<u16>,
    min: u16,
    max: u16,
    speed: f32,
) -> bool {
    let mut vi = [v.x as i32, v.y as i32];

    let value_changed = imgui::Drag::new(label)
        .range(min as i32, max as i32)
        .flags(imgui::SliderFlags::ALWAYS_CLAMP)
        .speed(speed)
        .build_array(ui, &mut vi);

    v.x = u16::try_from(vi[0]).unwrap();
    v.y = u16::try_from(vi[1]).unwrap();

    value_changed
}

/// Returns `true` if `film_settings` was changed.
fn generate_film_settings(ui: &imgui::Ui, film_settings: &mut FilmSettings) -> bool {
    let mut changed = false;
    ui.tree_node_config("Film").default_open(true).build(|| {
        changed |= vec2_u16_picker(
            ui,
            "Resolution",
            &mut film_settings.res,
            MIN_RES,
            MAX_RES,
            RES_STEP as f32,
        );

        {
            let _width = ui.push_item_width(118.0);
            changed |= u16_picker(
                ui,
                "Tile size",
                &mut film_settings.tile_dim,
                MIN_TILE,
                MIN_RES,
                TILE_STEP as f32,
            );
        }

        changed |= ui.checkbox("Accumulate", &mut film_settings.accumulate);
        changed |= ui.checkbox("Low res", &mut film_settings.sixteenth_res);
    });

    changed
}

fn generate_render_settings(ui: &imgui::Ui, render_settings: &mut RenderSettings) {
    ui.tree_node_config("Renderer")
        .default_open(true)
        .build(|| {
            ui.checkbox("Mark work tiles", &mut render_settings.mark_tiles);
            ui.checkbox(
                "Use single render thread",
                &mut render_settings.use_single_render_thread,
            );
        });
}

/// Returns `true` if `sampler` was changed.
fn generate_sampler_settings(ui: &imgui::Ui, sampler: &mut SamplerType) -> bool {
    let mut changed = false;
    ui.tree_node_config("Sampler").default_open(true).build(|| {
        changed |= enum_combo_box(ui, "##SamplerEnum", sampler);

        ui.indent();
        match sampler {
            SamplerType::Uniform(UniformParams { pixel_samples }) => {
                let _width = ui.push_item_width(118.0);
                changed |= u32_picker(
                    ui,
                    "Pixel extent samples",
                    pixel_samples,
                    1,
                    MAX_SAMPLES as u32,
                    1.0,
                );
            }
            SamplerType::Stratified(StratifiedParams {
                pixel_samples,
                symmetric_dimensions,
                jitter_samples,
            }) => {
                #[allow(clippy::cast_sign_loss)] // MAX_SAMPLES is u16
                let max_dim = f64::from(MAX_SAMPLES).sqrt() as u16;
                if *symmetric_dimensions {
                    let _width = ui.push_item_width(118.0);
                    changed |= u16_picker(
                        ui,
                        "Pixel extent samples",
                        &mut pixel_samples.x,
                        1,
                        max_dim,
                        1.0,
                    );
                    pixel_samples.y = pixel_samples.x;
                } else {
                    changed |= vec2_u16_picker(ui, "Pixel samples", pixel_samples, 1, max_dim, 1.0);
                }
                changed |= ui.checkbox("Symmetric dimensions", symmetric_dimensions);
                changed |= ui.checkbox("Jitter samples", jitter_samples);
                ui.text(format!(
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
    ui: &imgui::Ui,
    scene: &Scene,
    camera_params: &mut CameraParameters,
    load_settings: &mut SceneLoadSettings,
) -> bool {
    let mut changed = false;
    ui.tree_node_config("Scene").default_open(true).build(|| {
        ui.tree_node_config("Camera").default_open(true).build(|| {
            changed |= imgui::Drag::new("Position")
                .speed(0.1)
                .display_format("%.1f")
                .build_array(ui, camera_params.position.array_mut());

            changed |= imgui::Drag::new("Target")
                .speed(0.1)
                .display_format("%.1f")
                .build_array(ui, camera_params.target.array_mut());

            {
                let _width = ui.push_item_width(77.0);
                let fov = match &mut camera_params.fov {
                    FoV::X(ref mut v) | FoV::Y(ref mut v) => v,
                };
                changed |= imgui::Drag::new("Field of View")
                    .range(0.1, 359.9)
                    .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                    .speed(0.5)
                    .display_format("%.1f")
                    .build(ui, fov);
            }

            if ui.button("Set +Y up") {
                camera_params.up = Vec3::new(0.0, 1.0, 0.0);
                changed = true;
            }
        });

        ui.spacing();

        {
            enum_combo_box(ui, "##SplitMethodEnum", &mut load_settings.split_method);
            let _width = ui.push_item_width(92.0);
            u16_picker(
                ui,
                "Max shapes in BVH node",
                &mut load_settings.max_shapes_in_node,
                1,
                u16::max_value(),
                1.0,
            );
        }

        ui.spacing();

        if ui.button("Change scene") {
            let open_path = &scene.load_settings.path.to_str().unwrap();
            let path = open_file_dialog(
                "Open scene",
                open_path,
                Some((&["*.ply", "*.xml", "*.pbrt"], "Supported scene formats")),
            )
            .map_or_else(PathBuf::new, PathBuf::from);
            (*load_settings).path = path;
        }
        ui.same_line();
        if ui.button("Reload scene") {
            (*load_settings).path = scene.load_settings.path.clone();
        }
    });

    changed
}

/// Returns `true` if the integrator was changed.
fn generate_integrator_settings(ui: &imgui::Ui, integrator: &mut IntegratorType) -> bool {
    let mut changed = false;
    ui.tree_node_config("Integrator")
        .default_open(true)
        .build(|| {
            changed |= enum_combo_box(ui, "##IntegratorEnum", integrator);

            ui.indent();
            match integrator {
                IntegratorType::Whitted(WhittedParams { max_depth }) => {
                    let _width = ui.push_item_width(118.0);

                    changed |= imgui::Drag::new("Max depth##Integrator")
                        .range(1, u32::MAX)
                        .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                        .build(ui, max_depth);
                }
                IntegratorType::Path(PathParams {
                    max_depth,
                    indirect_clamp,
                }) => {
                    let _width = ui.push_item_width(118.0);

                    changed |= imgui::Drag::new("Max depth##Integrator")
                        .range(1, u32::MAX)
                        .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                        .build(ui, max_depth);

                    let mut clamp_active = indirect_clamp.is_some();
                    let clamp_changed =
                        ui.checkbox("Indirect clamp##Integrator", &mut clamp_active);
                    if clamp_changed {
                        if clamp_active {
                            *indirect_clamp = Some(2.0);
                        } else {
                            *indirect_clamp = None;
                        }
                        changed = true;
                    }
                    if let Some(c) = indirect_clamp.as_mut() {
                        changed |= imgui::Drag::new("##IntegratorIndirectClampSlider")
                            .range(1.0, 10.0)
                            .speed(0.1)
                            .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                            .build(ui, c);
                    }
                }
                IntegratorType::BVHIntersections
                | IntegratorType::GeometryNormals
                | IntegratorType::ShadingUVs
                | IntegratorType::ShadingNormals => (),
            }
            ui.unindent();
        });

    changed
}

fn generate_tone_map_settings(ui: &imgui::Ui, params: &mut ToneMapType) {
    ui.tree_node_config("Tone map")
        .default_open(true)
        .build(|| {
            enum_combo_box(ui, "##ToneMapEnum", params);
            ui.indent();
            match params {
                ToneMapType::Raw => (),
                ToneMapType::Filmic(FilmicParams { exposure }) => {
                    let _width = ui.push_item_width(118.0);
                    imgui::Drag::new("Exposure##ToneMap")
                        .range(0.0, f32::MAX)
                        .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                        .speed(0.001)
                        .display_format("%.3f")
                        .build(ui, exposure);
                }
                ToneMapType::Heatmap(HeatmapParams { bounds, channel }) => {
                    let changed = enum_combo_box(ui, "Channel##Heatmap", channel);
                    if changed {
                        *bounds = None;
                    }

                    if let Some((min, max)) = bounds {
                        let speed = ((*max - *min) / 100.0).max(0.001);
                        {
                            let _width = ui.push_item_width(118.0);
                            imgui::Drag::new("Min##Heatmap")
                                .range(0.0, (*max - 0.001).max(0.0))
                                .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                                .speed(speed)
                                .display_format("%.3f")
                                .build(ui, min);
                            ui.same_line();
                            imgui::Drag::new("Max##Heatmap")
                                .range(*min + 0.001, f32::MAX)
                                .flags(imgui::SliderFlags::ALWAYS_CLAMP)
                                .speed(speed)
                                .display_format("%.3f")
                                .build(ui, max);
                        }
                    }
                }
            }
            ui.unindent();
        });
}

// Generates a combo box for `value` and returns true if it changed.
fn enum_combo_box<T>(ui: &imgui::Ui, name: &str, value: &mut T) -> bool
where
    T: VariantNames + ToString + FromStr,
    T::Err: std::fmt::Debug,
{
    let mut current_t = T::VARIANTS
        .iter()
        .position(|&n| n == value.to_string())
        .unwrap();

    let changed = ui.combo_simple_string(name, &mut current_t, T::VARIANTS);

    if changed {
        *value = T::from_str(T::VARIANTS[current_t]).unwrap();
    }

    changed
}
