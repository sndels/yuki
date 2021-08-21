use crate::{
    materials::Bsdf,
    math::{Normal, Point3, Ray, Transform, Vec3},
};
use std::{convert::From, ops::Mul};

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Interactions#SurfaceInteraction

pub struct Interaction {
    pub p: Point3<f32>,
    pub n: Normal<f32>,
}

impl Default for Interaction {
    fn default() -> Self {
        Self {
            p: Point3::from(0.0),
            n: Normal::new(0.0, 0.0, 1.0),
        }
    }
}

impl Interaction {
    /// Spawns a ray from the `SurfaceInteraction` toward `d`.
    pub fn spawn_ray(&self, d: Vec3<f32>) -> Ray<f32> {
        let o = {
            // TODO: Base offset on p error
            let n = Vec3::from(self.n);
            let offset = n * 0.001;
            if d.dot(n) > 0.0 {
                self.p + offset
            } else {
                self.p - offset
            }
            // TODO: Round away from p
        };
        Ray::new(o, d, f32::INFINITY)
    }

    /// Spawns a ray from this `SurfaceInteraction` toward another one.
    /// Note that the ray direction is not normalized.
    pub fn spawn_ray_to(&self, other: &Interaction) -> Ray<f32> {
        let o = {
            // TODO: Base offset on p error
            let n = Vec3::from(self.n);
            let offset = n * 0.001;
            if (other.p - self.p).dot(n) > 0.0 {
                self.p + offset
            } else {
                self.p - offset
            }
            // TODO: Round away from p
        };
        // NOTE: This is not normalized
        let d = other.p - o;
        Ray::new(o, d, 0.9999)
    }
}

// Info for a point on a surface
pub struct SurfaceInteraction {
    /// World position
    pub p: Point3<f32>,
    pub dpdu: Vec3<f32>,
    pub dpdv: Vec3<f32>,
    /// View direction in world
    pub wo: Vec3<f32>,
    /// Surface normal
    pub n: Normal<f32>,
    /// Material
    pub bsdf: Option<Bsdf>,
}

impl SurfaceInteraction {
    pub fn new(
        p: Point3<f32>,
        dpdu: Vec3<f32>,
        dpdv: Vec3<f32>,
        wo: Vec3<f32>,
        should_reverse_normals: bool,
    ) -> Self {
        let n = {
            let mut n = Normal::from(dpdu.cross(dpdv).normalized());
            if should_reverse_normals {
                n *= -1.0;
            }
            n
        };
        Self {
            p,
            dpdu,
            dpdv,
            n,
            wo,
            bsdf: None,
        }
    }
}

impl<'a> Mul<SurfaceInteraction> for &'a Transform<f32> {
    type Output = SurfaceInteraction;

    fn mul(self, other: SurfaceInteraction) -> SurfaceInteraction {
        SurfaceInteraction {
            p: self * other.p,
            dpdu: self * other.dpdu,
            dpdv: self * other.dpdv,
            wo: self * other.wo,
            n: self * other.n,
            bsdf: other.bsdf,
        }
    }
}

impl From<&SurfaceInteraction> for Interaction {
    fn from(si: &SurfaceInteraction) -> Self {
        Self { p: si.p, n: si.n }
    }
}
