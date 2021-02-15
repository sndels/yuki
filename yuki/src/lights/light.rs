use crate::{hit::Hit, math::vector::Vec3};

/// Sample from a light source for visibility testing and shading
pub struct LightSample {
    pub l: Vec3<f32>,
    pub dist: f32,
    pub li: Vec3<f32>,
}

pub trait Light: Send + Sync {
    /// Returns a [LightSample] from `hit` to this `Light`.
    fn sample_li(&self, hit: &Hit) -> LightSample;
}
