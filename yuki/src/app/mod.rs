pub mod headless;
mod renderpasses;
mod ui;
mod util;
mod window;

pub use renderpasses::{FilmicParams, HeatmapParams, ToneMapType};
pub use window::Window;

use crate::{
    film::FilmSettings, integrators::IntegratorType, sampling::SamplerType,
    scene::SceneLoadSettings,
};

pub struct InitialSettings {
    pub film_settings: FilmSettings,
    pub sampler: SamplerType,
    pub scene_integrator: IntegratorType,
    pub tone_map: ToneMapType,
    pub load_settings: SceneLoadSettings,
}

impl Default for InitialSettings {
    fn default() -> Self {
        Self {
            film_settings: FilmSettings::default(),
            sampler: SamplerType::default(),
            scene_integrator: IntegratorType::default(),
            tone_map: ToneMapType::default(),
            load_settings: SceneLoadSettings::default(),
        }
    }
}
