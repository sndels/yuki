use allocators::ScopedScratch;
use std::sync::Arc;

use super::{mesh::Mesh, Hit, Shape};
use crate::{
    interaction::SurfaceInteraction,
    materials::{Bsdf, Material},
    math::{coordinate_system, Bounds3, Normal, Point2, Ray, Vec3},
};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Shapes/Triangle_Meshes.html

/// A triangle object.
pub struct Triangle {
    mesh: Arc<Mesh>,
    vertices: [usize; 3],
    material: Arc<dyn Material>,
}

impl Triangle {
    /// Creates a new `Triangle`.
    /// `first_vertex` is the index of the first vertex index in `mesh`'s index list.
    /// Expects counter clockwise winding
    pub fn new(mesh: Arc<Mesh>, first_vertex: usize, material: Arc<dyn Material>) -> Self {
        let vertices = [
            mesh.indices[first_vertex],
            mesh.indices[first_vertex + 1],
            mesh.indices[first_vertex + 2],
        ];

        Self {
            mesh,
            vertices,
            material,
        }
    }
}

impl Shape for Triangle {
    fn intersect(&self, ray: Ray<f32>) -> Option<Hit> {
        // pbrt's ray-triangle test performs the test in a coordinate space where the
        // ray lies on the +z axis. This way we don't get incorrect misses e.g. on rays
        // that intersect directly on an edge.

        let p0 = self.mesh.points[self.vertices[0]];
        let p1 = self.mesh.points[self.vertices[1]];
        let p2 = self.mesh.points[self.vertices[2]];

        let (p0t, p1t, p2t, sz) = {
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

            (p0t, p1t, p2t, sz)
        };

        // Edge coefficients
        let (e0, e1, e2) = {
            // No need for Z since we know d is on +Z
            let e0 = p1t.x * p2t.y - p1t.y * p2t.x;
            let e1 = p2t.x * p0t.y - p2t.y * p0t.x;
            let e2 = p0t.x * p1t.y - p0t.y * p1t.x;

            // Fall back to f64 if we're exactly on any edge
            if (e0 == 0.0) || (e1 == 0.0) || (e2 == 0.0) {
                let e0_64 = (p1t.x as f64) * (p2t.y as f64) - (p1t.y as f64) * (p2t.x as f64);
                let e1_64 = (p2t.x as f64) * (p0t.y as f64) - (p2t.y as f64) * (p0t.x as f64);
                let e2_64 = (p0t.x as f64) * (p1t.y as f64) - (p0t.y as f64) * (p1t.x as f64);
                (e0_64 as f32, e1_64 as f32, e2_64 as f32)
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
        let p0z_scaled = p0t.z * sz;
        let p1z_scaled = p1t.z * sz;
        let p2z_scaled = p2t.z * sz;
        let t_scaled = e0 * p0z_scaled + e1 * p1z_scaled + e2 * p2z_scaled;

        // Test against ray range
        if ((det < 0.0) && ((t_scaled >= 0.0) || (t_scaled < ray.t_max * det)))
            || ((det > 0.0) && ((t_scaled <= 0.0) || (t_scaled > ray.t_max * det)))
        {
            return None;
        }

        // Barycentric coordinates
        let inv_det = 1.0 / det;
        let b0 = e0 * inv_det;
        let b1 = e1 * inv_det;
        let b2 = e2 * inv_det;

        // World space distance to hit
        let t = t_scaled * inv_det;

        // Partial derivatives
        // TODO: Use mesh shading uvs if present
        let uv = [
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(1.0, 1.0),
        ];

        let duv02 = uv[0] - uv[2];
        let duv12 = uv[1] - uv[2];
        let dp02 = p0 - p2;
        let dp12 = p1 - p2;

        let uv_det = duv02[0] * duv12[1] - duv02[1] * duv12[0];
        let (dpdu, dpdv) = if uv_det == 0.0 {
            let n = (p2 - p0).cross(p1 - p0).normalized();
            coordinate_system(n)
        } else {
            let inv_uv_det = 1.0 / uv_det;
            (
                (dp02 * duv12[1] - dp12 * duv02[1]) * inv_uv_det,
                (-dp02 * duv12[0] + dp12 * duv02[0]) * inv_uv_det,
            )
        };

        let p_hit = p0 * b0 + p1 * b1 + p2 * b2;
        let mut si = SurfaceInteraction::new(p_hit, -ray.d, dpdu, dpdv, self);

        // Authored mesh UVs might not preserve orientation, but winding order is typically constant
        let n = Normal::from(dp02.cross(dp12).normalized());
        if self.transform_swaps_handedness() {
            si.n = -n;
            si.shading.n = -n;
        } else {
            si.n = n;
            si.shading.n = n;
        }

        // Set up shading normals
        if !self.mesh.normals.is_empty() {
            let n0 = self.mesh.normals[self.vertices[0]];
            let n1 = self.mesh.normals[self.vertices[1]];
            let n2 = self.mesh.normals[self.vertices[2]];

            let ns = {
                let n = Vec3::from(n0 * b0 + n1 * b1 + n2 * b2).normalized();
                if n.len_sqr() > 0.0 {
                    n.normalized()
                } else {
                    si.n.into()
                }
            };

            let (ss, ts) = {
                let mut ss = si.dpdu.normalized();
                let mut ts = ss.cross(ns);
                if ts.len_sqr() > 0.0 {
                    ts = ts.normalized();
                    ss = ts.cross(ns);
                    (ss, ts)
                } else {
                    coordinate_system(ns)
                }
            };

            si.set_shading_geometry(ss, ts);
        }

        Some(Hit { t, si, bsdf: None })
    }

    fn world_bound(&self) -> Bounds3<f32> {
        Bounds3::new(
            self.mesh.points[self.vertices[0]],
            self.mesh.points[self.vertices[1]],
        )
        .union_p(self.mesh.points[self.vertices[2]])
    }

    fn transform_swaps_handedness(&self) -> bool {
        self.mesh.transform_swaps_handedness
    }

    fn compute_scattering_functions<'a>(
        &self,
        scratch: &'a ScopedScratch,
        si: &SurfaceInteraction,
    ) -> Bsdf<'a> {
        self.material.compute_scattering_functions(scratch, si)
    }
}
