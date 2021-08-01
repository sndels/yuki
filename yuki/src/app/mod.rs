pub mod headless;
mod renderpasses;
mod ui;
mod util;
mod window;

pub use renderpasses::ToneMapType;
pub use window::Window;

use crate::{
    film::FilmSettings, integrators::IntegratorType, math::Vec2, samplers::SamplerSettings,
    scene::SceneLoadSettings,
};

pub struct InitialSettings {
    pub film_settings: FilmSettings,
    pub sampler_settings: SamplerSettings,
    pub scene_integrator: IntegratorType,
    pub tone_map: ToneMapType,
    pub load_settings: SceneLoadSettings,
}

impl Default for InitialSettings {
    fn default() -> Self {
        Self {
            film_settings: FilmSettings::default(),
            sampler_settings: SamplerSettings::StratifiedSampler {
                pixel_samples: Vec2::new(1, 1),
                symmetric_dimensions: true,
                jitter_samples: false,
            },
            scene_integrator: IntegratorType::default(),
            tone_map: ToneMapType::default(),
            load_settings: SceneLoadSettings::default(),
        }
    }
}
