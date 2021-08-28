use super::Sampler;
use crate::math::{Point2, Vec2};

use rand::{
    distributions::{Standard, Uniform},
    Rng,
};
use rand_pcg::Pcg32;

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Sampling_and_Reconstruction/Stratified_Sampling.html

pub struct StratifiedSampler {
    pixel_samples: Vec2<u16>,
    jitter_samples: bool,
    n_sampled_dimensions: usize,
    current_pixel_sample: usize,
    samples_2d: Vec<Vec<Point2<f32>>>,
    current_2d_dimension: usize,
    rng: Pcg32,
    // Stored to clone the sampler with a different stream
    rng_seed: u64,
}

impl StratifiedSampler {
    pub fn new(
        pixel_samples: Vec2<u16>,
        jitter_samples: bool,
        n_sampled_dimensions: usize,
    ) -> Self {
        // Known seed for debugging
        // let seed = 0x73B9642E74AC471C;
        // Random seed for normal use
        let seed = rand::thread_rng().gen();
        let total_pixel_samples = (pixel_samples.x as usize) * (pixel_samples.y as usize);
        Self {
            pixel_samples,
            jitter_samples,
            n_sampled_dimensions,
            current_pixel_sample: 0,
            samples_2d: vec![vec![Point2::from(0.0); total_pixel_samples]; n_sampled_dimensions],
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
                self.pixel_samples,
                self.jitter_samples,
                self.n_sampled_dimensions,
            )
        })
    }

    fn samples_per_pixel(&self) -> u32 {
        (self.pixel_samples.x as u32) * (self.pixel_samples.y as u32)
    }

    fn start_pixel(&mut self) {
        self.current_pixel_sample = 0;
        self.current_2d_dimension = 0;

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
        unimplemented!()
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
