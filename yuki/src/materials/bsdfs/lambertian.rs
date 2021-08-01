use super::{BxDF, BxdfSample, BxdfType};
use crate::math::Vec3;

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Reflection_Models/Lambertian_Reflection

pub struct Lambertian {
    reflectance: Vec3<f32>,
}

impl Lambertian {
    pub fn new(reflectance: Vec3<f32>) -> Self {
        Self { reflectance }
    }
}

impl BxDF for Lambertian {
    fn f(&self, _: Vec3<f32>, _: Vec3<f32>) -> Vec3<f32> {
        self.reflectance * std::f32::consts::FRAC_1_PI
    }

    fn sample_f(&self, _: Vec3<f32>) -> BxdfSample {
        unimplemented!()
    }

    fn flags(&self) -> BxdfType {
        BxdfType::DIFFUSE | BxdfType::REFLECTION
    }
}
