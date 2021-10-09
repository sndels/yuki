pub mod fresnel;
mod lambertian;
mod microfacet;
mod oren_nayar;
pub mod specular;

pub use lambertian::Lambertian;
pub use microfacet::{MicrofacetDistribution, MicrofacetReflection};
pub use oren_nayar::OrenNayar;

use crate::{
    interaction::SurfaceInteraction,
    math::{Normal, Point2, Spectrum, Vec3},
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
        const GLOSSY        = 0b01000;
        const SPECULAR      = 0b10000;
    }
}

pub struct BxdfSample {
    pub wi: Vec3<f32>,
    pub f: Spectrum<f32>,
    pub pdf: f32,
    pub sample_type: BxdfType,
}

impl Default for BxdfSample {
    fn default() -> Self {
        Self {
            wi: Vec3::from(0.0),
            f: Spectrum::zeros(),
            pdf: 0.0,
            sample_type: BxdfType::NONE,
        }
    }
}

/// Interface for an individual BRDF or BTDF function.
pub trait Bxdf {
    /// Evaluate distribution function for the pair of directions.
    fn f(&self, wo: Vec3<f32>, wi: Vec3<f32>) -> Spectrum<f32>;

    /// Returns an incident light diretion and the value of the `Bxdf` for the given outgoing direction
    fn sample_f(&self, wo: Vec3<f32>, u: Point2<f32>) -> BxdfSample;

    /// Evaluate probability distribution function for the pair of directions.
    fn pdf(&self, wo: Vec3<f32>, wi: Vec3<f32>) -> f32;

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
    n_shading: Normal<f32>,
    s_shading: Vec3<f32>,
    t_shading: Vec3<f32>,
}

impl Bsdf {
    /// Initializes `Bsdf` for given [`SurfaceInteraction`].
    /// Note that [`SurfaceInteraction::set_shading_geometry()`] should be called before this
    /// if shading geometry is different from the surface geometry
    pub fn new(si: &SurfaceInteraction) -> Self {
        let n_shading = si.shading.n;
        let s_shading = si.shading.dpdu.normalized();
        let t_shading = Vec3::from(n_shading).cross(s_shading);

        Self {
            bxdfs: Vec::new(),
            n_geom: si.n,
            n_shading,
            s_shading,
            t_shading,
        }
    }

    /// Adds 'bxdf' into this [`Bsdf`].
    pub fn add(&mut self, bxdf: Box<dyn Bxdf>) {
        self.bxdfs.push(bxdf);
    }

    /// Transform `v` from world space to surface local.
    fn world_to_local(&self, v: Vec3<f32>) -> Vec3<f32> {
        Vec3::new(
            v.dot(self.s_shading),
            v.dot(self.t_shading),
            v.dot_n(self.n_shading),
        )
    }

    /// Transform `v` from surface local to world space.
    fn local_to_world(&self, v: Vec3<f32>) -> Vec3<f32> {
        Vec3::new(
            self.s_shading.x * v.x + self.t_shading.x * v.y + self.n_shading.x * v.z,
            self.s_shading.y * v.x + self.t_shading.y * v.y + self.n_shading.y * v.z,
            self.s_shading.z * v.x + self.t_shading.z * v.y + self.n_shading.z * v.z,
        )
    }

    /// Evaluate distribution function for the pair of directions.
    pub fn f(
        &self,
        wo_world: Vec3<f32>,
        wi_world: Vec3<f32>,
        bxdf_type: BxdfType,
    ) -> Spectrum<f32> {
        let wo = self.world_to_local(wo_world);
        let wi = self.world_to_local(wi_world);

        let reflect = wi_world.dot_n(self.n_geom) * wo_world.dot_n(self.n_geom) > 0.0;

        let mut f = Spectrum::zeros();
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
    pub fn sample_f(
        &self,
        wo_world: Vec3<f32>,
        u: Point2<f32>,
        sample_type: BxdfType,
    ) -> BxdfSample {
        let matching_comps = self
            .bxdfs
            .iter()
            .filter(|bxdf| bxdf.matches(sample_type))
            .count();
        if matching_comps == 0 {
            return BxdfSample::default();
        }

        #[allow(clippy::cast_sign_loss)] // Always expect u in [0, 1)
        let comp = ((u[0] * (matching_comps as f32)).floor() as usize).min(matching_comps - 1);

        let bxdf = self
            .bxdfs
            .iter()
            .filter(|bxdf| bxdf.matches(sample_type))
            .nth(comp)
            .unwrap();

        let wo = self.world_to_local(wo_world);
        let u_remapped = Point2::new(u[0] * (matching_comps - comp) as f32, u[1]);

        let BxdfSample {
            wi: wi_local,
            mut f,
            mut pdf,
            sample_type: sampled_type,
        } = bxdf.sample_f(wo, u_remapped);
        if pdf == 0.0 {
            return BxdfSample::default();
        }
        let wi_world = self.local_to_world(wi_local);

        // TODO: Verify this once multiple non-specular lobes are used
        if !bxdf.flags().contains(BxdfType::SPECULAR) && matching_comps > 1 {
            for b in &self.bxdfs {
                if !std::ptr::eq(b as *const _, bxdf as *const _) && b.matches(sample_type) {
                    pdf += b.pdf(wo, wi_local);
                }
            }
        }
        if matching_comps > 1 {
            pdf /= matching_comps as f32;
        }

        // TODO: Verify this once multiple non-specular lobes are used
        if !bxdf.flags().contains(BxdfType::SPECULAR) && matching_comps > 1 {
            let reflect = wi_world.dot_n(self.n_geom) * wo_world.dot_n(self.n_geom) > 0.0;
            f = Spectrum::zeros();
            for b in &self.bxdfs {
                if b.matches(sample_type)
                    && ((reflect && b.flags().contains(BxdfType::REFLECTION))
                        || (!reflect && b.flags().contains(BxdfType::TRANSMISSION)))
                {
                    f += b.f(wo, wi_local);
                }
            }
        }

        // TODO: Verify type map makes sense for debug on 'all' query
        BxdfSample {
            wi: wi_world,
            f,
            pdf,
            sample_type: sampled_type,
        }
    }
}

fn cos_theta(w: Vec3<f32>) -> f32 {
    w.z
}

fn cos_2_theta(w: Vec3<f32>) -> f32 {
    w.z * w.z
}

fn sin_2_theta(w: Vec3<f32>) -> f32 {
    (1.0 - cos_2_theta(w)).max(0.0)
}

fn sin_theta(w: Vec3<f32>) -> f32 {
    sin_2_theta(w).sqrt()
}

fn tan_theta(w: Vec3<f32>) -> f32 {
    sin_theta(w) / cos_theta(w)
}

fn tan_2_theta(w: Vec3<f32>) -> f32 {
    sin_2_theta(w) / cos_2_theta(w)
}

fn sin_phi(w: Vec3<f32>) -> f32 {
    let sin_theta = sin_theta(w);
    if sin_theta == 0.0 {
        1.0
    } else {
        (w.y / sin_theta).clamp(-1.0, 1.0)
    }
}

fn sin_2_phi(w: Vec3<f32>) -> f32 {
    sin_phi(w) * sin_phi(w)
}

fn cos_phi(w: Vec3<f32>) -> f32 {
    let sin_theta = sin_theta(w);
    if sin_theta == 0.0 {
        1.0
    } else {
        (w.x / sin_theta).clamp(-1.0, 1.0)
    }
}

fn cos_2_phi(w: Vec3<f32>) -> f32 {
    cos_phi(w) * cos_phi(w)
}

fn same_hemisphere(w: Vec3<f32>, wp: Vec3<f32>) -> bool {
    w.z * wp.z > 0.0
}

fn spherical_direction(sin_theta: f32, cos_theta: f32, phi: f32) -> Vec3<f32> {
    Vec3::new(sin_theta * phi.cos(), sin_theta * phi.sin(), cos_theta)
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

fn reflect(wo: Vec3<f32>, n: Vec3<f32>) -> Vec3<f32> {
    -wo + n * 2.0 * wo.dot(n)
}
