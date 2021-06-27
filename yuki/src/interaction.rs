use crate::math::{Normal, Point3, Transform, Vec3};
use std::ops::Mul;

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Interactions#SurfaceInteraction

// Info for a point on a surface
#[derive(Clone)]
pub struct SurfaceInteraction {
    /// World position
    pub p: Point3<f32>,
    pub dpdu: Vec3<f32>,
    pub dpdv: Vec3<f32>,
    /// View direction in world
    pub wo: Vec3<f32>,
    /// Surface normal
    pub n: Normal<f32>,
    /// Diffuse surface color
    pub albedo: Vec3<f32>,
}

impl SurfaceInteraction {
    pub fn new(
        p: Point3<f32>,
        dpdu: Vec3<f32>,
        dpdv: Vec3<f32>,
        wo: Vec3<f32>,
        albedo: Vec3<f32>,
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
            albedo,
        }
    }
}

impl<'a> Mul<&SurfaceInteraction> for &'a Transform<f32> {
    type Output = SurfaceInteraction;

    fn mul(self, other: &SurfaceInteraction) -> SurfaceInteraction {
        SurfaceInteraction {
            p: self * other.p,
            dpdu: self * other.dpdu,
            dpdv: self * other.dpdv,
            wo: self * other.wo,
            n: self * other.n,
            albedo: other.albedo,
        }
    }
}
