use super::{
    bsdfs::{Bsdf, Lambertian},
    Material,
};
use crate::{interaction::SurfaceInteraction, math::Spectrum, textures::Texture};

use std::sync::Arc;

pub struct Matte {
    kd: Arc<dyn Texture<Spectrum<f32>>>,
    // TODO: sigma (roughness) with oren-nayar
}

impl Matte {
    pub fn new(kd: Arc<dyn Texture<Spectrum<f32>>>) -> Self {
        Self { kd }
    }
}

impl Material for Matte {
    fn compute_scattering_functions(&self, si: &SurfaceInteraction) -> Bsdf {
        let mut bsdf = Bsdf::new(si);

        let reflectance = self.kd.evaluate(si);
        bsdf.add(Box::new(Lambertian::new(reflectance)));

        bsdf
    }
}
