use super::{
    bsdfs::{fresnel, specular, Bsdf},
    Material,
};
use crate::{interaction::SurfaceInteraction, math::Vec3};

pub struct Glass {
    r: Vec3<f32>,
    t: Vec3<f32>,
    eta: f32,
}

impl Glass {
    pub fn new(r: Vec3<f32>, t: Vec3<f32>, eta: f32) -> Self {
        Self { r, t, eta }
    }
}

impl Material for Glass {
    fn compute_scattering_functions(&self, si: &SurfaceInteraction) -> Bsdf {
        let mut bsdf = Bsdf::new(si);

        bsdf.add(Box::new(specular::Reflection::new(
            self.r,
            Box::new(fresnel::Dielectric::new(1.0, self.eta)),
        )));

        bsdf.add(Box::new(specular::Transmission::new(self.t, 1.0, self.eta)));

        bsdf
    }
}
