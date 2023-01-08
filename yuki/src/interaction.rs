use crate::{
    lights::AreaLight,
    math::{Normal, Point2, Point3, Ray, Spectrum, Transform, Vec3},
    shapes::Shape,
};
use std::{ops::Mul, sync::Arc};

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Geometry_and_Transformations/Interactions#SurfaceInteraction

pub struct Interaction {
    pub p: Point3<f32>,
    pub n: Normal<f32>,
}

impl Default for Interaction {
    fn default() -> Self {
        Self {
            p: Point3::zeros(),
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

pub struct ShadingGeometry {
    pub n: Normal<f32>,
    pub dpdu: Vec3<f32>,
    pub dpdv: Vec3<f32>,
}

impl<'a> Mul<&ShadingGeometry> for &'a Transform<f32> {
    type Output = ShadingGeometry;

    fn mul(self, other: &ShadingGeometry) -> ShadingGeometry {
        ShadingGeometry {
            n: (self * other.n).normalized(),
            dpdu: self * other.dpdu,
            dpdv: self * other.dpdv,
        }
    }
}

// Info for a point on a surface
pub struct SurfaceInteraction {
    pub p: Point3<f32>,
    pub n: Normal<f32>,
    pub uv: Point2<f32>,
    pub dpdu: Vec3<f32>,
    pub dpdv: Vec3<f32>,
    pub shading: ShadingGeometry,
    pub wo: Vec3<f32>,
    shape_transform_swaps_handedness: bool,
    pub area_light: Option<Arc<dyn AreaLight>>,
}

impl SurfaceInteraction {
    /// Creates a new `SurfaceInteraction` with its surface geometry populated and shading geometry initialized to match the surface geometry.
    pub fn new(
        p: Point3<f32>,
        wo: Vec3<f32>,
        uv: Point2<f32>,
        dpdu: Vec3<f32>,
        dpdv: Vec3<f32>,
        shape: &dyn Shape,
        area_light: Option<Arc<dyn AreaLight>>,
    ) -> Self {
        let shape_transform_swaps_handedness = shape.transform_swaps_handedness();
        let n = {
            let n = Normal::from(dpdu.cross(dpdv).normalized());
            if shape_transform_swaps_handedness {
                -n
            } else {
                n
            }
        };
        Self {
            p,
            n,
            uv,
            dpdu,
            dpdv,
            shading: ShadingGeometry { n, dpdu, dpdv },
            wo,
            shape_transform_swaps_handedness,
            area_light,
        }
    }

    pub fn set_shading_geometry(&mut self, dpdus: Vec3<f32>, dpdvs: Vec3<f32>) {
        self.shading.n = Normal::from(dpdus.cross(dpdvs)).normalized();
        self.n = self.n.faceforward_n(self.shading.n);

        self.shading.dpdu = dpdus;
        self.shading.dpdv = dpdvs;
    }

    pub fn emitted_radiance(&self, w: Vec3<f32>) -> Spectrum<f32> {
        self.area_light
            .as_ref()
            .map_or(Spectrum::zeros(), |l| l.radiance(self, w))
    }
}

impl<'a> Mul<SurfaceInteraction> for &'a Transform<f32> {
    type Output = SurfaceInteraction;

    fn mul(self, other: SurfaceInteraction) -> SurfaceInteraction {
        let n = (self * other.n).normalized();
        let mut shading = self * &other.shading;
        shading.n = shading.n.faceforward_n(n);

        let mut ret = SurfaceInteraction {
            p: self * other.p,
            n,
            uv: other.uv,
            dpdu: self * other.dpdu,
            dpdv: self * other.dpdv,
            wo: (self * other.wo).normalized(),
            shading,
            area_light: other.area_light,
            shape_transform_swaps_handedness: other.shape_transform_swaps_handedness,
        };
        ret.shading.n = ret.shading.n.faceforward_n(ret.n);

        ret
    }
}

impl From<&SurfaceInteraction> for Interaction {
    fn from(si: &SurfaceInteraction) -> Self {
        Self { p: si.p, n: si.n }
    }
}
