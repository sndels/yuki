use super::Sampler;
use crate::{
    hash_values,
    math::{Point2, Vec2},
};

use rand::{distributions::Standard, Rng};
use rand_pcg::Pcg32;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

// Ported with tweaks from pbrt-v4
// https://github.com/mmp/pbrt-v4/blob/master/src/pbrt/samplers.h
// Pbrt-v3 had pre-computed samples per pixel
// http://www.pbr-book.org/3ed-2018/Sampling_and_Reconstruction/Stratified_Sampling.html
// but this generates each sample on the fly like in
// https://graphics.pixar.com/library/MultiJitteredSampling/paper.pdf

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct Params {
    pub pixel_samples: Vec2<u16>,
    pub symmetric_dimensions: bool,
    pub jitter_samples: bool,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            pixel_samples: Vec2::new(1, 1),
            symmetric_dimensions: true,
            jitter_samples: true,
        }
    }
}

pub struct StratifiedSampler {
    pixel_samples: Vec2<u16>,
    jitter_samples: bool,
    pixel: Point2<u16>,
    sample_index: u32,
    dimension: u32,
    rng: Pcg32,
    rng_seed: u64,
}

impl StratifiedSampler {
    pub fn new(mut params: Params, force_single_sample: bool) -> Self {
        // Known seed for debugging
        // let seed = 0x73B9642E74AC471C;
        // Random seed for normal use
        let seed = rand::thread_rng().gen();

        if force_single_sample {
            params.pixel_samples = Vec2::new(1, 1);
        }

        Self {
            pixel_samples: params.pixel_samples,
            jitter_samples: params.jitter_samples,
            pixel: Point2::new(0, 0),
            sample_index: 0,
            dimension: 0,
            rng: Pcg32::new(seed, 0),
            rng_seed: seed,
        }
    }
}

impl Sampler for StratifiedSampler {
    fn clone(&self) -> Box<dyn Sampler> {
        Box::new(Self {
            // Different streams are used for pixels so literally clone the sampler
            rng: Pcg32::new(self.rng_seed, 0),
            rng_seed: self.rng_seed,
            ..Self::new(
                Params {
                    pixel_samples: self.pixel_samples,
                    symmetric_dimensions: false,
                    jitter_samples: self.jitter_samples,
                },
                false,
            )
        })
    }

    fn samples_per_pixel(&self) -> u32 {
        (self.pixel_samples.x as u32) * (self.pixel_samples.y as u32)
    }

    fn start_pixel_sample(&mut self, p: Point2<u16>, index: u32, dimension: u32) {
        self.pixel = p;
        self.sample_index = index;
        self.dimension = 0;

        let hashed = hash_values!(self.pixel);
        // pbrt hashes the pixel and rng_seed together, using that for stream and
        // a mixed version for seed. selecting stream based on pixel hash also seems
        // valid as streams for the same seed should be uncorrelated
        self.rng = Pcg32::new(self.rng_seed, hashed);
        self.rng
            .advance((self.sample_index as u64) * 65536u64 + (dimension as u64));
    }

    fn get_1d(&mut self) -> f32 {
        let hashed = hash_values!(self.pixel, self.dimension, self.rng_seed);
        let stratum = permutation_element(
            self.sample_index as u32,
            self.samples_per_pixel(),
            hashed as u32,
        );

        self.dimension += 1;
        let delta = if self.jitter_samples {
            self.rng.sample(Standard)
        } else {
            0.5
        };
        ((stratum as f32) + delta) / (self.samples_per_pixel() as f32)
    }

    fn get_2d(&mut self) -> Point2<f32> {
        let hashed = hash_values!(self.pixel, self.dimension, self.rng_seed);
        let stratum =
            permutation_element(self.sample_index, self.samples_per_pixel(), hashed as u32);

        self.dimension += 2;
        let x = stratum % (self.pixel_samples.x as u32);
        let y = stratum / (self.pixel_samples.y as u32);
        let dx = if self.jitter_samples {
            self.rng.sample(Standard)
        } else {
            0.5
        };
        let dy = if self.jitter_samples {
            self.rng.sample(Standard)
        } else {
            0.5
        };
        Point2::new(
            ((x as f32) + dx) / (self.pixel_samples.x as f32),
            ((y as f32) + dy) / (self.pixel_samples.y as f32),
        )
    }
}

// This appears to be from https://graphics.pixar.com/library/MultiJitteredSampling/paper.pdf
fn permutation_element(mut i: u32, l: u32, p: u32) -> u32 {
    let mut w = l - 1;
    w |= w >> 1;
    w |= w >> 2;
    w |= w >> 4;
    w |= w >> 8;
    w |= w >> 16;
    loop {
        i ^= p;
        i = i.wrapping_mul(0xe170893d);
        i ^= p >> 16;
        i ^= (i & w) >> 4;
        i ^= p >> 8;
        i = i.wrapping_mul(0x0929eb3f);
        i ^= p >> 23;
        i ^= (i & w) >> 1;
        i = i.wrapping_mul(1 | p >> 27);
        i = i.wrapping_mul(0x6935fa69);
        i ^= (i & w) >> 11;
        i = i.wrapping_mul(0x74dcb303);
        i ^= (i & w) >> 2;
        i = i.wrapping_mul(0x9e501cc3);
        i ^= (i & w) >> 2;
        i = i.wrapping_mul(0xc860a3df);
        i &= w;
        i ^= i >> 5;
        if i < l {
            break;
        }
    }
    (i + p) % l
}
