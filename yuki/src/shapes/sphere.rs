use allocators::ScopedScratch;
use std::sync::Arc;

use super::{Hit, Shape};
use crate::{
    interaction::SurfaceInteraction,
    materials::{Bsdf, Material},
    math::{Bounds3, Point2, Point3, Ray, Transform, Vec3},
};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Shapes/Spheres.htmll

/// A sphere object.
pub struct Sphere {
    object_to_world: Transform<f32>,
    world_to_object: Transform<f32>,
    radius: f32,
    material: Arc<dyn Material>,
    transform_swaps_handedness: bool,
}

impl Sphere {
    /// Creates a new `Sphere`.
    pub fn new(object_to_world: &Transform<f32>, radius: f32, material: Arc<dyn Material>) -> Self {
        Self {
            object_to_world: object_to_world.clone(),
            world_to_object: object_to_world.inverted(),
            radius,
            material,
            transform_swaps_handedness: object_to_world.swaps_handedness(),
        }
    }
}

impl Shape for Sphere {
    #[allow(clippy::many_single_char_names)] // Convention
    fn intersect(&self, ray: Ray<f32>) -> Option<Hit> {
        let r = &self.world_to_object * ray;

        // Quadratic coefficients
        let a = r.d.x * r.d.x + r.d.y * r.d.y + r.d.z * r.d.z;
        let b = 2.0 * (r.d.x * r.o.x + r.d.y * r.o.y + r.d.z * r.o.z);
        let c = r.o.x * r.o.x + r.o.y * r.o.y + r.o.z * r.o.z - self.radius * self.radius;

        // Solve quadratic equation for ts
        let discrim = b * b - 4.0 * a * c;
        if discrim < 0.0 {
            return None;
        }
        let rd = discrim.sqrt();

        let q = if b < 0.0 {
            -0.5 * (b - rd)
        } else {
            -0.5 * (b + rd)
        };

        // Find hit points
        let mut t0 = q / a;
        let mut t1 = c / q;
        if t0 > t1 {
            std::mem::swap(&mut t0, &mut t1);
        }

        if t0 > r.t_max || t1 <= 0.0 {
            return None;
        }
        let mut t = t0;
        if t <= 0.0 {
            t = t1;
            if t > r.t_max {
                return None;
            }
        };

        // Do in object space to compute parametric representation
        let p = {
            let mut p = r.point(t);
            // Refine
            p *= self.radius / p.dist(Point3::zeros());
            // Remove division by zero further on
            if p.x == 0.0 && p.y == 0.0 {
                p.x = 1e-5_f32 * self.radius;
            }
            p
        };

        let phi = {
            let mut phi = p.y.atan2(p.x);
            if phi < 0.0 {
                phi += 2.0 * std::f32::consts::PI;
            }
            phi
        };

        // TODO: Simplify math
        let phi_max = 2.0 * std::f32::consts::PI;
        let theta_min = std::f32::consts::PI;
        let theta_max = 0.0;
        let u = phi / phi_max;
        let theta = (p.z / self.radius).clamp(-1.0, 1.0).acos();
        let v = (theta - theta_min) / (theta_max - theta_min);

        let (dpdu, dpdv) = {
            let z_radius = (p.x * p.x + p.y * p.y).sqrt();
            let inv_z_radius = 1.0 / z_radius;
            let cos_phi = p.x * inv_z_radius;
            let sin_phi = p.y * inv_z_radius;
            let dpdu = Vec3::new(-phi_max * p.y, phi_max * p.x, 0.0);
            let dpdv = Vec3::new(p.z * cos_phi, p.z * sin_phi, -self.radius * theta.sin())
                * (theta_max - theta_min);
            (dpdu, dpdv)
        };
        let si = &self.object_to_world
            * SurfaceInteraction::new(p, -ray.d, Point2::new(u, v), dpdu, dpdv, self, None);

        Some(Hit { t, si, shape: self })
    }

    fn world_bound(&self) -> Bounds3<f32> {
        &self.object_to_world * Bounds3::new(Point3::from(-self.radius), Point3::from(self.radius))
    }

    fn transform_swaps_handedness(&self) -> bool {
        self.transform_swaps_handedness
    }

    fn compute_scattering_functions<'a>(
        &self,
        scratch: &'a ScopedScratch,
        si: &SurfaceInteraction,
    ) -> Bsdf<'a> {
        self.material.compute_scattering_functions(scratch, si)
    }
}
