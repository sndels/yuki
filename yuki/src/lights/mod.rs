mod point_light;
mod spot_light;

pub use point_light::PointLight;
pub use spot_light::SpotLight;

use crate::{interaction::SurfaceInteraction, math::Vec3};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Light_Sources/Light_Interface.html#Light

/// Sample from a light source for visibility testing and shading
pub struct LightSample {
    pub l: Vec3<f32>,
    pub dist: f32,
    pub li: Vec3<f32>,
}

pub trait Light: Send + Sync {
    /// Returns a [LightSample] from the given [SurfaceInteraction] to this `Light`.
    fn sample_li(&self, si: &SurfaceInteraction) -> LightSample;
}
