use super::{
    bsdfs::{fresnel, specular, Bsdf},
    Material,
};
use crate::{interaction::SurfaceInteraction, math::Spectrum, textures::Texture};

use allocators::ScopedScratch;
use std::sync::Arc;

pub struct Glass {
    r: Arc<dyn Texture<Spectrum<f32>>>,
    t: Arc<dyn Texture<Spectrum<f32>>>,
    eta: f32,
}

impl Glass {
    pub fn new(
        r: Arc<dyn Texture<Spectrum<f32>>>,
        t: Arc<dyn Texture<Spectrum<f32>>>,
        eta: f32,
    ) -> Self {
        Self { r, t, eta }
    }
}

impl Material for Glass {
    fn compute_scattering_functions<'a>(
        &self,
        scratch: &'a ScopedScratch,
        si: &SurfaceInteraction,
    ) -> Bsdf<'a> {
        let mut bsdf = Bsdf::new(si);
        bsdf.add(scratch.alloc(specular::Reflection::new(
            self.r.evaluate(si),
            scratch.alloc(fresnel::Dielectric::new(1.0, self.eta)),
        )));

        bsdf.add(scratch.alloc(specular::Transmission::new(
            self.t.evaluate(si),
            1.0,
            self.eta,
        )));

        bsdf
    }
}
