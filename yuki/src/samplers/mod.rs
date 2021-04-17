pub mod stratified;

use crate::math::{point::Point2, vector::Vec2};
use std::sync::Arc;

#[derive(Copy, Clone)]
pub enum SamplerSettings {
    StratifiedSampler {
        pixel_samples: Vec2<u16>,
        symmetric_dimensions: bool,
        jitter_samples: bool,
    },
}

pub fn create_sampler(settings: SamplerSettings) -> Arc<dyn Sampler> {
    Arc::new(match settings {
        SamplerSettings::StratifiedSampler {
            pixel_samples,
            jitter_samples,
            ..
        } => stratified::StratifiedSampler::new(pixel_samples, jitter_samples),
    })
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
