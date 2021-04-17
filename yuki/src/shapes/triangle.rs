use std::sync::Arc;

use super::{mesh::Mesh, Shape};
use crate::{
    hit::Hit,
    math::{bounds::Bounds3, normal::Normal, ray::Ray, vector::Vec3},
};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Shapes/Triangle_Meshes.html

/// A triangle object.
pub struct Triangle {
    mesh: Arc<Mesh>,
    vertices: [usize; 3],
    albedo: Vec3<f32>,
}

impl Triangle {
    /// Creates a new `Triangle`.
    /// `first_vertex` is the index of the first vertex index in `mesh`'s index list.
    /// Expects counter clockwise winding
    pub fn new(mesh: Arc<Mesh>, first_vertex: usize, albedo: Vec3<f32>) -> Self {
        let vertices = [
            mesh.indices[first_vertex],
            mesh.indices[first_vertex + 1],
            mesh.indices[first_vertex + 2],
        ];

        Self {
            mesh,
            vertices,
            albedo,
        }
    }
}

impl Shape for Triangle {
    fn intersect(&self, ray: Ray<f32>) -> Option<Hit> {
        // pbrt's ray-triangle test performs the test in a coordinate space where the
        // ray lies on the +z axis. This way we don't get incorrect misses e.g. on rays
        // that intersect directly on an edge.

        let (mut n, p0t, p1t, p2t, sz) = {
            let p0 = self.mesh.points[self.vertices[0]];
            let p1 = self.mesh.points[self.vertices[1]];
            let p2 = self.mesh.points[self.vertices[2]];

            let n = Normal::from((p1 - p0).cross(p2 - p0).normalized());

            // Do things in relation to ray's origin
            let mut p0t = p0 - ray.o;
            let mut p1t = p1 - ray.o;
            let mut p2t = p2 - ray.o;

            // Permute direction so that Z is largest
            // This ensures there is a non-zero magnitude on Z
            let kz = ray.d.abs().max_dimension();
            let kx = if kz < 2 { kz + 1 } else { 0 };
            let ky = if kx < 2 { kx + 1 } else { 0 };
            p0t = p0t.permuted(kx, ky, kz);
            p1t = p1t.permuted(kx, ky, kz);
            p2t = p2t.permuted(kx, ky, kz);
            let d = ray.d.permuted(kx, ky, kz);

            // Shear to get +Z forward
            // Defer shearing Z since we won't need it if we don't intersect
            // TODO: The shear factors could be pre-computed for each ray
            let sx = -d.x / d.z;
            let sy = -d.y / d.z;
            let sz = 1.0 / d.z;
            p0t.x += sx * p0t.z;
            p0t.y += sy * p0t.z;
            p1t.x += sx * p1t.z;
            p1t.y += sy * p1t.z;
            p2t.x += sx * p2t.z;
            p2t.y += sy * p2t.z;

            (n, p0t, p1t, p2t, sz)
        };

        // Edge coefficients
        let (e0, e1, e2) = {
            // No need for Z since we know d is on +Z
            let e0 = p1t.x * p2t.y - p1t.y * p2t.x;
            let e1 = p2t.x * p0t.y - p2t.y * p0t.x;
            let e2 = p0t.x * p1t.y - p0t.y * p1t.x;

            // Fall back to f64 if we're exactly on any edge
            if (e0 == 0.0) || (e1 == 0.0) || (e2 == 0.0) {
                let e0 = (p1t.x as f64) * (p2t.y as f64) - (p1t.y as f64) * (p2t.x as f64);
                let e1 = (p2t.x as f64) * (p0t.y as f64) - (p2t.y as f64) * (p0t.x as f64);
                let e2 = (p0t.x as f64) * (p1t.y as f64) - (p0t.y as f64) * (p1t.x as f64);
                (e0 as f32, e1 as f32, e2 as f32)
            } else {
                (e0, e1, e2)
            }
        };

        // Edge test, i.e. if we miss the triangle
        if ((e0 < 0.0) || (e1 < 0.0) || (e2 < 0.0)) && ((e0 > 0.0) || (e1 > 0.0) || (e2 > 0.0)) {
            return None;
        }

        // Determinant test, i.e. if we hit the triangle edge-on
        let det = e0 + e1 + e2;
        if det == 0.0 {
            return None;
        }

        // Scaled hit distance
        let p0z = p0t.z * sz;
        let p1z = p1t.z * sz;
        let p2z = p2t.z * sz;
        let t_scaled = e0 * p0z + e1 * p1z + e2 * p2z;

        // Test against ray range
        if ((det < 0.0) && ((t_scaled >= 0.0) || (t_scaled < ray.t_max * det)))
            || ((det > 0.0) && ((t_scaled <= 0.0) || (t_scaled > ray.t_max * det)))
        {
            return None;
        }

        // World space distance to hit
        let t = t_scaled / det;

        // Flip normal for backface hits
        n = if ray.d.dot_n(n) < 0.0 { n } else { -n };

        // pbrt swaps normal direction if object_to_world swaps handedness.
        // We won't need to since our normal is already calculated with world space
        // vertex positions.

        Some(Hit {
            t,
            p: ray.point(t),
            v: -ray.d,
            n,
            albedo: self.albedo,
        })
    }

    fn world_bound(&self) -> Bounds3<f32> {
        Bounds3::new(
            self.mesh.points[self.vertices[0]],
            self.mesh.points[self.vertices[1]],
        )
        .union_p(self.mesh.points[self.vertices[2]])
    }
}
