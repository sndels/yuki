use crate::math::normal::Normal;

pub struct Hit {
    pub t: f32,
    pub n: Normal<f32>,
}
