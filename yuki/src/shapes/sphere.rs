use super::Shape;
use crate::{
    hit::Hit,
    math::{Bounds3, Normal, Point3, Ray, Transform, Vec3},
};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Shapes/Spheres.htmll

/// A sphere object.
pub struct Sphere {
    object_to_world: Transform<f32>,
    world_to_object: Transform<f32>,
    radius: f32,
    albedo: Vec3<f32>,
}

impl Sphere {
    /// Creates a new `Sphere`.
    pub fn new(object_to_world: &Transform<f32>, radius: f32, albedo: Vec3<f32>) -> Self {
        Self {
            object_to_world: object_to_world.clone(),
            world_to_object: object_to_world.inverted(),
            radius,
            albedo,
        }
    }
}

impl Shape for Sphere {
    fn intersect(&self, ray: Ray<f32>) -> Option<Hit> {
        let Ray { o, d, t_max } = &self.world_to_object * ray;

        // Quadratic coefficients
        let a = d.x * d.x + d.y * d.y + d.z * d.z;
        let b = 2.0 * (d.x * o.x + d.y * o.y + d.z * o.z);
        let c = o.x * o.x + o.y * o.y + o.z * o.z - self.radius * self.radius;

        // Solve quadratic equation for ts
        let d = b * b - 4.0 * a * c;
        if d < 0.0 {
            return None;
        }
        let rd = d.sqrt();

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

        if t0 > t_max || t1 <= 0.0 {
            return None;
        }
        let mut t = t0;
        if t <= 0.0 {
            t = t1;
            if t > t_max {
                return None;
            }
        };

        // TODO: This can be computed the same way for all surfaces from the partial derivatives dp/du, dp/dv
        let n = Normal::from((ray.point(t) - &self.object_to_world * Point3::zeros()).normalized());

        Some(Hit {
            t,
            p: ray.point(t),
            v: -ray.d,
            n,
            albedo: self.albedo,
        })
    }

    fn world_bound(&self) -> Bounds3<f32> {
        &self.object_to_world * Bounds3::new(Point3::from(-self.radius), Point3::from(self.radius))
    }
}
