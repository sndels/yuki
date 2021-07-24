use super::{
    bsdfs::{Bsdf, Lambertian},
    Material,
};
use crate::{interaction::SurfaceInteraction, math::Vec3};

pub struct Matte {
    // TODO: This should be a texture
    kd: Vec3<f32>,
    // TODO: sigma (roughness) with oren-nayar
}

impl Matte {
    pub fn new(kd: Vec3<f32>) -> Self {
        Self { kd }
    }
}

impl Material for Matte {
    fn compute_scattering_functions(&self, si: &SurfaceInteraction) -> Bsdf {
        let mut bsdf = Bsdf::new(si);

        bsdf.add(Box::new(Lambertian::new(self.kd)));

        bsdf
    }
}
