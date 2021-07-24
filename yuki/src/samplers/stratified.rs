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
    current_pixel_sample: usize,
    samples_2d: Vec<Point2<f32>>,
    current_2d_dimension: usize,
    rng: Pcg32,
    // Stored to clone the sampler with a different stream
    rng_seed: u64,
}

impl StratifiedSampler {
    pub fn new(pixel_samples: Vec2<u16>, jitter_samples: bool) -> Self {
        // Known seed for debugging
        // let seed = 0x73B9642E74AC471C;
        // Random seed for normal use
        let seed = rand::thread_rng().gen();
        Self {
            pixel_samples,
            jitter_samples,
            current_pixel_sample: 0,
            samples_2d: Vec::new(),
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
            ..Self::new(self.pixel_samples, self.jitter_samples)
        })
    }

    fn samples_per_pixel(&self) -> u32 {
        (self.pixel_samples.x as u32) * (self.pixel_samples.y as u32)
    }

    fn start_pixel(&mut self) {
        self.current_pixel_sample = 0;
        self.current_2d_dimension = 0;

        // TODO: Measure effect of writing new values in place vs reinitializing each pixel
        self.samples_2d.clear();

        let samples = stratified_sample_2d(self.pixel_samples, self.jitter_samples, &mut self.rng);
        self.samples_2d = samples;
        shuffle(&mut self.samples_2d, &mut self.rng);
    }

    fn start_sample(&mut self) {
        self.current_pixel_sample += 1;
        self.current_2d_dimension = 0;
    }

    fn get_1d(&mut self) -> f32 {
        unimplemented!()
    }

    fn get_2d(&mut self) -> Point2<f32> {
        if self.current_2d_dimension > 0 {
            unimplemented!("Single precalc 2D dimension only")
        }
        // Start sample adds 1 before we use the sample
        let ret = self.samples_2d[self.current_pixel_sample - 1];
        self.current_2d_dimension += 1;
        ret
    }
}

const ONE_MINUS_EPSILON: f32 = 1.0_f32 - f32::EPSILON;

fn stratified_sample_2d(n_samples: Vec2<u16>, jitter: bool, rng: &mut Pcg32) -> Vec<Point2<f32>> {
    let d = Vec2::new(1.0 / (n_samples.x as f32), 1.0 / (n_samples.y as f32));
    (0..n_samples.y)
        .flat_map(|y| {
            (0..n_samples.x)
                .map(|x| {
                    let (jx, jy) = if jitter {
                        (rng.sample(Standard), rng.sample(Standard))
                    } else {
                        (0.5, 0.5)
                    };
                    Point2::new(
                        (((x as f32) + jx) * d.x).min(ONE_MINUS_EPSILON),
                        (((y as f32) + jy) * d.y).min(ONE_MINUS_EPSILON),
                    )
                })
                .collect::<Vec<Point2<f32>>>()
        })
        .collect()
}

fn shuffle<T>(samples: &mut Vec<T>, rng: &mut Pcg32) {
    for i in 0..samples.len() {
        let other = i + rng.sample(Uniform::from(0..(samples.len() - i)));
        samples.swap(i, other);
    }
}
