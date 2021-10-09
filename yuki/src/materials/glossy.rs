use super::{
    bsdfs::{fresnel, Bsdf, MicrofacetReflection, TrowbridgeReitzDistribution},
    Material,
};
use crate::{interaction::SurfaceInteraction, math::Spectrum, textures::Texture};

use std::sync::Arc;

// Approximates Blender's Glossy BSDF
pub struct Glossy {
    rs: Arc<dyn Texture<Spectrum<f32>>>,
    roughness: Arc<dyn Texture<f32>>,
    remap_roughness: bool,
}

impl Glossy {
    pub fn new(
        rs: Arc<dyn Texture<Spectrum<f32>>>,
        roughness: Arc<dyn Texture<f32>>,
        remap_roughness: bool,
    ) -> Self {
        Self {
            rs,
            roughness,
            remap_roughness,
        }
    }
}

impl Material for Glossy {
    fn compute_scattering_functions(&self, si: &SurfaceInteraction) -> Bsdf {
        let mut bsdf = Bsdf::new(si);

        let roughness = if self.remap_roughness {
            TrowbridgeReitzDistribution::roughness_to_alpha(self.roughness.evaluate(si))
        } else {
            self.roughness.evaluate(si)
        };

        let rs = self.rs.evaluate(si);

        let fresnel = fresnel::Schlick::new(rs);
        // Squared roughness seems to mirror how Blender's shader node behaves
        let distribution = TrowbridgeReitzDistribution::new(roughness * roughness);

        bsdf.add(Box::new(MicrofacetReflection::new(
            Spectrum::new(1.0, 1.0, 1.0),
            Box::new(distribution),
            Box::new(fresnel),
        )));

        bsdf
    }
}
