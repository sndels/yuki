use super::Integrator;
use crate::{
    math::{Ray, Vec3},
    scene::Scene,
};

/// The first channel of returned color is the number of BVH intersections performed.
/// The second channel is the number of BVH node hits found.
/// The third channel is the number of BVH node hits found if the ray also hit scene geometry.
pub struct BVHIntersectionsIntegrator;

impl Integrator for BVHIntersectionsIntegrator {
    fn li(ray: Ray<f32>, scene: &Scene) -> (Vec3<f32>, usize) {
        let (hit, (bvh_intersection_count, bvh_hit_count)) = scene.bvh.intersect(ray);
        let ray_count = 1;

        let color = Vec3::new(
            bvh_intersection_count as f32,
            bvh_hit_count as f32,
            if hit.is_some() {
                bvh_hit_count as f32
            } else {
                0.0
            },
        );

        (color, ray_count)
    }
}
