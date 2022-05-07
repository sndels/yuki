mod distant_light;
mod point_light;
mod rectangular_light;
mod spot_light;

pub use distant_light::DistantLight;
pub use point_light::PointLight;
pub use rectangular_light::RectangularLight;
pub use spot_light::SpotLight;

use crate::{
    interaction::SurfaceInteraction,
    math::{Point2, Spectrum, Vec3},
    visibility::VisibilityTester,
};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Light_Sources/Light_Interface.html#Light
// https://pbr-book.org/3ed-2018/Light_Sources/Area_Lights

/// Sample from a light source for visibility testing and shading
pub struct LightSample {
    pub l: Vec3<f32>,
    pub li: Spectrum<f32>,
    pub vis: Option<VisibilityTester>,
    pub pdf: f32,
}

pub trait Light: Send + Sync {
    /// Returns a [`LightSample`] from the given [`SurfaceInteraction`] to this [`Light`].
    fn sample_li(&self, si: &SurfaceInteraction, u: Point2<f32>) -> LightSample;
}

pub trait AreaLight: Send + Sync {
    /// Returns the emitted radiance in the direction `w`.
    fn radiance(&self, si: &SurfaceInteraction, w: Vec3<f32>) -> Spectrum<f32>;
}
