use super::Sampler;
use crate::math::{Point2, Vec2};

use rand::{
    distributions::{Standard, Uniform},
    Rng,
};
use rand_pcg::Pcg32;
use serde::{Deserialize, Serialize};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Sampling_and_Reconstruction/Stratified_Sampling.html

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
    n_sampled_dimensions: usize,
    current_pixel_sample: usize,
    samples_1d: Vec<Vec<f32>>,
    samples_2d: Vec<Vec<Point2<f32>>>,
    current_1d_dimension: usize,
    current_2d_dimension: usize,
    rng: Pcg32,
    // Stored to clone the sampler with a different stream
    rng_seed: u64,
}

impl StratifiedSampler {
    pub fn new(params: Params, n_sampled_dimensions: usize) -> Self {
        // Known seed for debugging
        // let seed = 0x73B9642E74AC471C;
        // Random seed for normal use
        let seed = rand::thread_rng().gen();
        let total_pixel_samples =
            (params.pixel_samples.x as usize) * (params.pixel_samples.y as usize);
        Self {
            pixel_samples: params.pixel_samples,
            jitter_samples: params.jitter_samples,
            n_sampled_dimensions,
            current_pixel_sample: 0,
            samples_1d: vec![vec![0.0; total_pixel_samples]; n_sampled_dimensions],
            samples_2d: vec![vec![Point2::from(0.0); total_pixel_samples]; n_sampled_dimensions],
            current_1d_dimension: 0,
            current_2d_dimension: 0,
            rng: Pcg32::new(seed, 0),
            rng_seed: seed,
        }
    }
}

impl Sampler for StratifiedSampler {
    fn clone(&self, seed: u64) -> Box<dyn Sampler> {
        Box::new(Self {
            // Pcg has uncorrelated streams so let's leverage that
            rng: Pcg32::new(self.rng_seed, seed),
            ..Self::new(
                Params {
                    pixel_samples: self.pixel_samples,
                    symmetric_dimensions: false,
                    jitter_samples: self.jitter_samples,
                },
                self.n_sampled_dimensions,
            )
        })
    }

    fn samples_per_pixel(&self) -> u32 {
        (self.pixel_samples.x as u32) * (self.pixel_samples.y as u32)
    }

    fn start_pixel(&mut self) {
        self.current_pixel_sample = 0;
        self.current_1d_dimension = 0;
        self.current_2d_dimension = 0;

        for dim_samples in &mut self.samples_1d {
            stratified_sample_1d(dim_samples, self.jitter_samples, &mut self.rng);
            shuffle(dim_samples, &mut self.rng);
        }

        for dim_samples in &mut self.samples_2d {
            stratified_sample_2d(
                dim_samples,
                self.pixel_samples,
                self.jitter_samples,
                &mut self.rng,
            );
            shuffle(dim_samples, &mut self.rng);
        }
    }

    fn start_sample(&mut self) {
        self.current_pixel_sample += 1;
        self.current_2d_dimension = 0;
    }

    fn get_1d(&mut self) -> f32 {
        if self.current_1d_dimension < self.n_sampled_dimensions {
            // Start sample adds 1 before we use the sample
            let ret = self.samples_1d[self.current_1d_dimension][self.current_pixel_sample - 1];
            self.current_1d_dimension += 1;
            ret
        } else {
            self.rng.sample(Standard)
        }
    }

    fn get_2d(&mut self) -> Point2<f32> {
        if self.current_2d_dimension < self.n_sampled_dimensions {
            // Start sample adds 1 before we use the sample
            let ret = self.samples_2d[self.current_2d_dimension][self.current_pixel_sample - 1];
            self.current_2d_dimension += 1;
            ret
        } else {
            Point2::new(self.rng.sample(Standard), self.rng.sample(Standard))
        }
    }
}

const ONE_MINUS_EPSILON: f32 = 1.0_f32 - f32::EPSILON;

fn stratified_sample_1d(samples: &mut [f32], jitter: bool, rng: &mut Pcg32) {
    let inv_n_samples = 1.0 / (samples.len() as f32);
    for (i, sample) in samples.iter_mut().enumerate() {
        let delta = if jitter { rng.sample(Standard) } else { 0.5 };
        *sample = (((i as f32) + delta) * inv_n_samples).min(ONE_MINUS_EPSILON);
    }
}

fn stratified_sample_2d(
    samples: &mut [Point2<f32>],
    n_samples: Vec2<u16>,
    jitter: bool,
    rng: &mut Pcg32,
) {
    let d = Vec2::new(1.0 / (n_samples.x as f32), 1.0 / (n_samples.y as f32));
    for y in 0..n_samples.y as usize {
        let row_index = y * (n_samples.x as usize);
        for x in 0..n_samples.x as usize {
            let (jx, jy) = if jitter {
                (rng.sample(Standard), rng.sample(Standard))
            } else {
                (0.5, 0.5)
            };
            let index = row_index + x;
            samples[index] = Point2::new(
                (((x as f32) + jx) * d.x).min(ONE_MINUS_EPSILON),
                (((y as f32) + jy) * d.y).min(ONE_MINUS_EPSILON),
            );
        }
    }
}

fn shuffle<T>(samples: &mut Vec<T>, rng: &mut Pcg32) {
    for i in 0..samples.len() {
        let other = i + rng.sample(Uniform::from(0..(samples.len() - i)));
        samples.swap(i, other);
    }
}
