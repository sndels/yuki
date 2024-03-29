use super::{
    bsdfs::{fresnel, Bsdf, MicrofacetReflection, TrowbridgeReitzDistribution},
    Material,
};
use crate::{interaction::SurfaceInteraction, math::Spectrum, textures::Texture};

use allocators::ScopedScratch;
use std::sync::Arc;

pub struct Metal {
    eta: Arc<dyn Texture<Spectrum<f32>>>,
    k: Arc<dyn Texture<Spectrum<f32>>>,
    roughness: Arc<dyn Texture<f32>>,
    remap_roughness: bool,
}

impl Metal {
    pub fn new(
        eta: Arc<dyn Texture<Spectrum<f32>>>,
        k: Arc<dyn Texture<Spectrum<f32>>>,
        roughness: Arc<dyn Texture<f32>>,
        remap_roughness: bool,
    ) -> Self {
        Self {
            eta,
            k,
            roughness,
            remap_roughness,
        }
    }
}

impl Material for Metal {
    fn compute_scattering_functions<'a>(
        &self,
        scratch: &'a ScopedScratch,
        si: &SurfaceInteraction,
    ) -> Bsdf<'a> {
        let mut bsdf = Bsdf::new(si);

        let roughness = if self.remap_roughness {
            TrowbridgeReitzDistribution::roughness_to_alpha(self.roughness.evaluate(si))
        } else {
            self.roughness.evaluate(si)
        };

        let fresnel = fresnel::Conductor::new(
            Spectrum::new(1.0, 1.0, 1.0),
            self.eta.evaluate(si),
            self.k.evaluate(si),
        );
        let distribution = TrowbridgeReitzDistribution::new(roughness);

        bsdf.add(scratch.alloc(MicrofacetReflection::new(
            Spectrum::new(1.0, 1.0, 1.0),
            scratch.alloc(distribution),
            scratch.alloc(fresnel),
        )));

        bsdf
    }
}
