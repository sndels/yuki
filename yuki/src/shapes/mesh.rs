use crate::math::{Point3, Transform};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Light_Sources/Point_Lights.html

/// Stores the geometry data of a triangle mesh
pub struct Mesh {
    pub object_to_world: Transform<f32>,
    /// Triangle vertex indices stored as triplets
    pub indices: Vec<usize>,
    /// Points in world space
    pub points: Vec<Point3<f32>>,
}

impl Mesh {
    /// Creates a new `Mesh`
    pub fn new(
        object_to_world: &Transform<f32>,
        indices: Vec<usize>,
        mut points: Vec<Point3<f32>>,
    ) -> Self {
        for p in &mut points {
            *p = object_to_world * *p;
        }

        Self {
            object_to_world: object_to_world.clone(),
            indices,
            points,
        }
    }
}
