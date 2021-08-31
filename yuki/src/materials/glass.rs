use super::{
    bsdfs::{fresnel, specular, Bsdf},
    Material,
};
use crate::{interaction::SurfaceInteraction, math::Spectrum, textures::Texture};

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
    fn compute_scattering_functions(&self, si: &SurfaceInteraction) -> Bsdf {
        let mut bsdf = Bsdf::new(si);
        bsdf.add(Box::new(specular::Reflection::new(
            self.r.evaluate(si),
            Box::new(fresnel::Dielectric::new(1.0, self.eta)),
        )));

        bsdf.add(Box::new(specular::Transmission::new(
            self.t.evaluate(si),
            1.0,
            self.eta,
        )));

        bsdf
    }
}
