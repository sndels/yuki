mod stratified;
mod uniform;

pub use stratified::StratifiedSampler;
pub use uniform::UniformSampler;

pub type StratifiedParams = stratified::Params;
pub type UniformParams = uniform::Params;

use crate::math::{Point2, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use strum::{Display, EnumString, EnumVariantNames};

#[derive(Copy, Clone, Deserialize, Serialize, Display, EnumVariantNames, EnumString)]
pub enum SamplerType {
    Uniform(uniform::Params),
    Stratified(stratified::Params),
}

impl SamplerType {
    pub fn instantiate(self, force_single_sample: bool) -> Arc<dyn Sampler> {
        match self {
            SamplerType::Stratified(params) => {
                Arc::new(StratifiedSampler::new(params, force_single_sample))
            }
            SamplerType::Uniform(params) => {
                Arc::new(UniformSampler::new(params, force_single_sample))
            }
        }
    }
}

#[allow(clippy::derivable_impls)] // Can't derive Default for non unit variants, which Stratifed is
impl Default for SamplerType {
    fn default() -> Self {
        SamplerType::Stratified(stratified::Params::default())
    }
}

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Sampling_and_Reconstruction/Sampling_Interface.html
// and pbrt-v4
// https://github.com/mmp/pbrt-v4

pub trait Sampler: Send + Sync {
    /// Clones a `Sampler` with the given prng `seed`.
    fn clone(&self) -> Box<dyn Sampler>;
    /// Returns the number of samples per pixel this `Sampler` generates.
    fn samples_per_pixel(&self) -> u32;
    /// Readies the sampler for a new pixel sample.
    fn start_pixel_sample(&mut self, p: Point2<u16>, sample_index: u32, dimension: u32);
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
    if offset == Point2::zeros() {
        return Point2::zeros();
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

#[macro_export]
macro_rules! hash_values {
    ($($v:expr),+) => {{
        // TODO:
        // memcpy to a small u8 buffer and hashing that might be faster, but also
        // much messier.
        // Perf difference seems to currently be around 3% in total time on a 5900x,
        // rendering the default cornell box at 720p, fov 65 and 256 samples.
        let mut hasher = std::collections::hash_map::DefaultHasher::default();
        $(
            $v.hash(&mut hasher);
        )+
        hasher.finish()
    }};
}
