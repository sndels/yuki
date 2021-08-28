use super::{cos_theta, same_hemisphere, Bxdf, BxdfSample, BxdfType};
use crate::{
    math::{Point2, Vec3},
    sampling::cosine_sample_hemisphere,
};

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

impl Bxdf for Lambertian {
    fn f(&self, _: Vec3<f32>, _: Vec3<f32>) -> Vec3<f32> {
        self.reflectance * std::f32::consts::FRAC_1_PI
    }

    fn sample_f(&self, wo: Vec3<f32>, u: Point2<f32>) -> BxdfSample {
        let mut wi = cosine_sample_hemisphere(u);
        if wo.z < 0.0 {
            wi.z *= -1.0;
        };

        let pdf = self.pdf(wo, wi);
        let f = self.f(wo, wi);

        BxdfSample {
            wi,
            f,
            pdf,
            sample_type: self.flags(),
        }
    }

    fn pdf(&self, wo: Vec3<f32>, wi: Vec3<f32>) -> f32 {
        if same_hemisphere(wo, wi) {
            cos_theta(wi).abs() * std::f32::consts::FRAC_1_PI
        } else {
            0.0
        }
    }

    fn flags(&self) -> BxdfType {
        BxdfType::DIFFUSE | BxdfType::REFLECTION
    }
}
