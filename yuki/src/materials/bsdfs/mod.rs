pub mod fresnel;
mod lambertian;
pub mod specular;

pub use lambertian::Lambertian;

use crate::{
    interaction::SurfaceInteraction,
    math::{Normal, Vec3},
};

use bitflags::bitflags;

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Materials/BSDFs
// https://www.pbr-book.org/3ed-2018/Reflection_Models/Basic_Interface#BxDF

bitflags! {
    pub struct BxdfType: u8 {
        const NONE          = 0b00000;
        const REFLECTION    = 0b00001;
        const TRANSMISSION  = 0b00010;
        const DIFFUSE       = 0b00100;
        const SPECULAR      = 0b01000;
    }
}

pub struct BxdfSample {
    pub wi: Vec3<f32>,
    pub f: Vec3<f32>,
    pub sample_type: BxdfType,
}

/// Interface for an individual BRDF or BTDF function.
pub trait Bxdf {
    /// Evaluate distribution function for the pair of directions.
    fn f(&self, wo: Vec3<f32>, wi: Vec3<f32>) -> Vec3<f32>;

    /// Returns an incident light diretion and the value of the `Bxdf` for the given outgoing direction
    fn sample_f(&self, wo: Vec3<f32>) -> BxdfSample;

    /// Returns the type flags for this `Bxdf`
    fn flags(&self) -> BxdfType;

    /// Returns `true` if the `Bxdf` matches the given type
    fn matches(&self, t: BxdfType) -> bool {
        t.contains(self.flags())
    }
}

/// A collection of BxDF functions.
pub struct Bsdf {
    bxdfs: Vec<Box<dyn Bxdf>>,
    n_geom: Normal<f32>,
    // TODO: Shading normal
    // TODO: These should be s_shading, t_shading
    s_geom: Vec3<f32>,
    t_geom: Vec3<f32>,
}

impl Bsdf {
    pub fn new(si: &SurfaceInteraction) -> Self {
        let n_geom = si.n;
        // TODO: These should be from shading uv partial derivatives in relation to shading normal once uvs are implemented
        let s_geom = si.dpdu.normalized();
        let t_geom = Vec3::from(n_geom).cross(s_geom);

        Self {
            bxdfs: Vec::new(),
            n_geom,
            s_geom,
            t_geom,
        }
    }

    /// Adds 'bxdf' into this [`Bsdf`].
    pub fn add(&mut self, bxdf: Box<dyn Bxdf>) {
        self.bxdfs.push(bxdf);
    }

    /// Transform `v` from world space to surface local.
    fn world_to_local(&self, v: Vec3<f32>) -> Vec3<f32> {
        Vec3::new(v.dot(self.s_geom), v.dot(self.t_geom), v.dot_n(self.n_geom))
    }

    /// Transform `v` from surface local to world space.
    fn local_to_world(&self, v: Vec3<f32>) -> Vec3<f32> {
        Vec3::new(
            self.s_geom.x * v.x + self.t_geom.x * v.y + self.n_geom.x * v.z,
            self.s_geom.y * v.x + self.t_geom.y * v.y + self.n_geom.y * v.z,
            self.s_geom.z * v.x + self.t_geom.z * v.y + self.n_geom.z * v.z,
        )
    }

    /// Evaluate distribution function for the pair of directions.
    pub fn f(&self, wo_world: Vec3<f32>, wi_world: Vec3<f32>, bxdf_type: BxdfType) -> Vec3<f32> {
        let wo = self.world_to_local(wo_world);
        let wi = self.world_to_local(wi_world);

        let reflect = wi_world.dot_n(self.n_geom) * wo_world.dot_n(self.n_geom) > 0.0;

        let mut f = Vec3::from(0.0);
        for bxdf in &self.bxdfs {
            if bxdf.matches(bxdf_type)
                && ((reflect && bxdf.flags().contains(BxdfType::REFLECTION))
                    || (!reflect && bxdf.flags().contains(BxdfType::TRANSMISSION)))
            {
                f += bxdf.f(wo, wi);
            }
        }

        f
    }

    /// Samples the first `Bxdf` matching `sample_type`.
    pub fn sample_f(&self, wo_world: Vec3<f32>, sample_type: BxdfType) -> BxdfSample {
        // TODO: Materials with multiple matching lobes
        assert!(
            self.bxdfs
                .iter()
                .filter(|bxdf| bxdf.matches(sample_type))
                .count()
                <= 1,
            "Sampling Bsdf with multiple matching lobes"
        );

        self.bxdfs
            .iter()
            .find(|bxdf| bxdf.matches(sample_type))
            .map_or_else(
                || BxdfSample {
                    wi: Vec3::from(0.0),
                    f: Vec3::from(0.0),
                    sample_type: BxdfType::NONE,
                },
                |bxdf| {
                    let wo = self.world_to_local(wo_world);

                    let mut ret = bxdf.sample_f(wo);
                    ret.wi = self.local_to_world(ret.wi);

                    ret
                },
            )
    }
}

fn cos_theta(w: Vec3<f32>) -> f32 {
    w.z
}

// Returns the refracted direction for `wi` and `n` or `None` if total internal reflection happens.
fn refract(wi: Vec3<f32>, n: Normal<f32>, eta: f32) -> Option<Vec3<f32>> {
    let cos_theta_i = n.dot_v(wi);
    let sin_2_theta_i = (1.0 - cos_theta_i * cos_theta_i).max(0.0);
    let sin_2_theta_t = eta * eta * sin_2_theta_i;

    let total_internal_reflection = sin_2_theta_t >= 1.0;
    if total_internal_reflection {
        return None;
    }

    let cos_theta_t = (1.0 - sin_2_theta_t).sqrt();
    Some(-wi * eta + Vec3::from(n) * (eta * cos_theta_i - cos_theta_t))
}
