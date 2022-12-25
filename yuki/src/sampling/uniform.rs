use super::Sampler;
use crate::{hash_values, math::Point2};

use rand::{distributions::Standard, Rng};
use rand_pcg::Pcg32;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Sampling_and_Reconstruction/Stratified_Sampling.html

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct Params {
    pub pixel_samples: u32,
}

impl Default for Params {
    fn default() -> Self {
        Self { pixel_samples: 1 }
    }
}

pub struct UniformSampler {
    pixel_samples: u32,
    pixel: Point2<u16>,
    sample_index: u32,
    dimension: u32,
    rng: Pcg32,
    rng_seed: u64,
}

impl UniformSampler {
    pub fn new(mut params: Params, force_single_sample: bool) -> Self {
        // Known seed for debugging
        // let seed = 0x73B9642E74AC471C;
        // Random seed for normal use
        let seed = rand::thread_rng().gen();

        if force_single_sample {
            params.pixel_samples = 1;
        }

        Self {
            pixel_samples: params.pixel_samples,
            pixel: Point2::new(0, 0),
            sample_index: 0,
            dimension: 0,
            rng: Pcg32::new(seed, 0),
            rng_seed: seed,
        }
    }
}
impl Sampler for UniformSampler {
    fn clone(&self) -> Box<dyn Sampler> {
        Box::new(Self {
            // Different streams are used for pixels so literally clone the sampler
            rng: Pcg32::new(self.rng_seed, 0),
            rng_seed: self.rng_seed,
            ..Self::new(
                Params {
                    pixel_samples: self.pixel_samples,
                },
                false,
            )
        })
    }

    fn samples_per_pixel(&self) -> u32 {
        self.pixel_samples as u32
    }

    fn start_pixel_sample(&mut self, p: Point2<u16>, sample_index: u32, dimension: u32) {
        self.pixel = p;
        self.sample_index = sample_index;
        self.dimension = dimension;

        let hashed = hash_values!(self.pixel);
        // pbrt hashes the pixel and rng_seed together, using that for stream and
        // a mixed version for seed. selecting stream based on pixel hash also seems
        // valid as streams for the same seed should be uncorrelated
        self.rng = Pcg32::new(self.rng_seed, hashed);
        self.rng
            .advance((self.sample_index as u64) * 65536u64 + (dimension as u64));
    }

    fn get_1d(&mut self) -> f32 {
        self.dimension += 1;
        self.rng.sample(Standard)
    }

    fn get_2d(&mut self) -> Point2<f32> {
        self.dimension += 2;
        Point2::new(self.rng.sample(Standard), self.rng.sample(Standard))
    }
}
