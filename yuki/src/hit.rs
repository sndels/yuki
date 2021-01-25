use crate::math::{normal::Normal, vector::Vec3};

pub struct Hit {
    pub t: f32,
    pub n: Normal<f32>,
    pub albedo: Vec3<f32>,
}
