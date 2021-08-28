mod stratified;
mod uniform;

pub use stratified::StratifiedSampler;
pub use uniform::UniformSampler;

pub type StratifiedParams = stratified::Params;
pub type UniformParams = uniform::Params;

use crate::math::{Point2, Vec2, Vec3};
use std::sync::Arc;
use strum::{EnumString, EnumVariantNames, ToString};

#[derive(Copy, Clone, EnumVariantNames, ToString, EnumString)]
pub enum SamplerType {
    Uniform(uniform::Params),
    Stratified(stratified::Params),
}

impl SamplerType {
    pub fn instantiate(self, n_sampled_dimensions: usize) -> Arc<dyn Sampler> {
        match self {
            SamplerType::Stratified(params) => {
                Arc::new(StratifiedSampler::new(params, n_sampled_dimensions))
            }
            SamplerType::Uniform(params) => {
                Arc::new(UniformSampler::new(params, n_sampled_dimensions))
            }
        }
    }
}

impl Default for SamplerType {
    fn default() -> Self {
        SamplerType::Stratified(stratified::Params::default())
    }
}

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Sampling_and_Reconstruction/Sampling_Interface.html

pub trait Sampler: Send + Sync {
    /// Clones a `Sampler` with the given prng `seed`.
    fn clone(&self, seed: u64) -> Box<dyn Sampler>;
    /// Returns the number of samples per pixel this `Sampler` generates.
    fn samples_per_pixel(&self) -> u32;
    /// Readies the sampler for a new pixel.
    fn start_pixel(&mut self);
    /// Readies the sampler for a new pixel sample.
    fn start_sample(&mut self);
    /// Returns the next dimension in the current sample vector.
    fn get_1d(&mut self) -> f32;
    /// Returns the next two dimensions in the current sample vector.
    fn get_2d(&mut self) -> Point2<f32>;
}

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Monte_Carlo_Integration/2D_Sampling_with_Multidimensional_Transformations

pub fn cosine_sample_hemisphere(u: Point2<f32>) -> Vec3<f32> {
    let d = concentric_sample_disk(u);
    let z = (1.0 - d.x * d.x - d.y * d.y).max(0.0).sqrt();
    Vec3::new(d.x, d.y, z)
}

pub fn concentric_sample_disk(u: Point2<f32>) -> Point2<f32> {
    let offset = u * 2.0 - Vec2::new(1.0, 1.0);
    if offset == Point2::from(0.0) {
        return Point2::from(0.0);
    }

    let (theta, r) = if offset.x.abs() > offset.y.abs() {
        (
            std::f32::consts::FRAC_PI_4 * (offset.y / offset.x),
            offset.x,
        )
    } else {
        (
            std::f32::consts::FRAC_PI_2 - std::f32::consts::FRAC_PI_4 * (offset.x / offset.y),
            offset.y,
        )
    };

    Point2::new(theta.cos(), theta.sin()) * r
}
