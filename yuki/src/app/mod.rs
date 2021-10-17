pub mod headless;
mod renderpasses;
mod ui;
mod util;
mod window;

pub use renderpasses::{FilmicParams, HeatmapParams, ToneMapType};
pub use window::Window;

use crate::{
    film::FilmSettings, integrators::IntegratorType, renderer::RenderSettings,
    sampling::SamplerType, scene::SceneLoadSettings,
};

use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize)]
pub struct InitialSettings {
    pub film_settings: Option<FilmSettings>,
    pub render_settings: Option<RenderSettings>,
    pub sampler: Option<SamplerType>,
    pub scene_integrator: Option<IntegratorType>,
    pub tone_map: Option<ToneMapType>,
    pub load_settings: Option<SceneLoadSettings>,
}
