use super::{
    bsdfs::{Bsdf, Lambertian, OrenNayar},
    Material,
};
use crate::{interaction::SurfaceInteraction, math::Spectrum, textures::Texture};

use allocators::ScopedScratch;
use std::sync::Arc;

pub struct Matte {
    kd: Arc<dyn Texture<Spectrum<f32>>>,
    sigma: Arc<dyn Texture<f32>>,
}

impl Matte {
    pub fn new(kd: Arc<dyn Texture<Spectrum<f32>>>, sigma: Arc<dyn Texture<f32>>) -> Self {
        Self { kd, sigma }
    }
}

impl Material for Matte {
    fn compute_scattering_functions<'a>(
        &self,
        scratch: &'a ScopedScratch,
        si: &SurfaceInteraction,
    ) -> Bsdf<'a> {
        let mut bsdf = Bsdf::new(si);

        let reflectance = self.kd.evaluate(si);
        let sigma = self.sigma.evaluate(si);
        if !reflectance.is_black() {
            if sigma == 0.0 {
                bsdf.add(scratch.alloc(Lambertian::new(reflectance)));
            } else {
                bsdf.add(scratch.alloc(OrenNayar::new(reflectance, sigma)));
            }
        }

        bsdf
    }
}
