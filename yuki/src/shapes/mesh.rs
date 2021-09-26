use crate::math::{Normal, Point3, Transform};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Light_Sources/Point_Lights.html

/// Stores the geometry data of a triangle mesh
pub struct Mesh {
    pub object_to_world: Transform<f32>,
    /// Triangle vertex indices stored as triplets
    pub indices: Vec<usize>,
    /// Points in world space
    pub points: Vec<Point3<f32>>,
    pub normals: Vec<Normal<f32>>,
    pub transform_swaps_handedness: bool,
}

impl Mesh {
    /// Creates a new `Mesh`
    pub fn new(
        object_to_world: &Transform<f32>,
        indices: Vec<usize>,
        mut points: Vec<Point3<f32>>,
        mut normals: Vec<Normal<f32>>,
    ) -> Self {
        for p in &mut points {
            *p = object_to_world * *p;
        }

        for n in &mut normals {
            *n = object_to_world * *n;
        }

        Self {
            object_to_world: object_to_world.clone(),
            indices,
            points,
            normals,
            transform_swaps_handedness: object_to_world.swaps_handedness(),
        }
    }
}
