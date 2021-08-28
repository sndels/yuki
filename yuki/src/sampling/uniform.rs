use super::Sampler;
use crate::math::Point2;

use rand::{distributions::Standard, Rng};
use rand_pcg::Pcg32;

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Sampling_and_Reconstruction/Stratified_Sampling.html

pub struct UniformSampler {
    pixel_samples: u32,
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

impl UniformSampler {
    pub fn new(pixel_samples: u32, n_sampled_dimensions: usize) -> Self {
        // Known seed for debugging
        // let seed = 0x73B9642E74AC471C;
        // Random seed for normal use
        let seed = rand::thread_rng().gen();
        Self {
            pixel_samples,
            n_sampled_dimensions,
            current_pixel_sample: 0,
            samples_1d: vec![vec![0.0; pixel_samples as usize]; n_sampled_dimensions],
            samples_2d: vec![vec![Point2::from(0.0); pixel_samples as usize]; n_sampled_dimensions],
            current_1d_dimension: 0,
            current_2d_dimension: 0,
            rng: Pcg32::new(seed, 0),
            rng_seed: seed,
        }
    }
}
impl Sampler for UniformSampler {
    fn clone(&self, seed: u64) -> Box<dyn Sampler> {
        Box::new(Self {
            // Pcg has uncorrelated streams so let's leverage that
            rng: Pcg32::new(self.rng_seed, seed),
            ..Self::new(self.pixel_samples, self.n_sampled_dimensions)
        })
    }

    fn samples_per_pixel(&self) -> u32 {
        self.pixel_samples as u32
    }

    fn start_pixel(&mut self) {
        self.current_pixel_sample = 0;
        self.current_1d_dimension = 0;
        self.current_2d_dimension = 0;

        for dim_samples in &mut self.samples_1d {
            for sample in dim_samples {
                *sample = self.rng.sample(Standard);
            }
        }

        for dim_samples in &mut self.samples_2d {
            for sample in dim_samples {
                *sample = Point2::new(self.rng.sample(Standard), self.rng.sample(Standard));
            }
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
