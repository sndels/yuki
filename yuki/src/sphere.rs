use crate::math::{ray::Ray, transform::Transform};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Shapes/Spheres.htmll

/// A sphere object.
pub struct Sphere {
    world_to_object: Transform<f32>,
    radius: f32,
}

impl Sphere {
    /// Creates a new `Sphere`.
    pub fn new(object_to_world: &Transform<f32>, radius: f32) -> Self {
        Self {
            world_to_object: object_to_world.inverted(),
            radius,
        }
    }

    /// Checks for [Ray] intersection with this `Sphere`.
    pub fn intersect(&self, ray: Ray<f32>) -> Option<f32> {
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

        Some(t)
    }
}
