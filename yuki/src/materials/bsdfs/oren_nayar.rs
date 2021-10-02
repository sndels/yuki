use super::{cos_phi, cos_theta, same_hemisphere, sin_phi, sin_theta, Bxdf, BxdfSample, BxdfType};
use crate::{
    math::{Point2, Spectrum, Vec3},
    sampling::cosine_sample_hemisphere,
};

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Reflection_Models/Microfacet_Models#OrenndashNayarDiffuseReflection

pub struct OrenNayar {
    reflectance: Spectrum<f32>,
    a: f32,
    b: f32,
}

impl OrenNayar {
    /// Instead of pbrt's sigma degrees, expect sigma radians.
    pub fn new(reflectance: Spectrum<f32>, sigma: f32) -> Self {
        let sigma2 = sigma * sigma;
        let a = 1.0 - (sigma2 / (2.0 * (sigma2 + 0.33)));
        let b = 0.45 * sigma2 / (sigma2 + 0.09);
        Self { reflectance, a, b }
    }
}

impl Bxdf for OrenNayar {
    fn f(&self, wi: Vec3<f32>, wo: Vec3<f32>) -> Spectrum<f32> {
        let sin_theta_i = sin_theta(wi);
        let sin_theta_o = sin_theta(wo);

        let max_cos = if sin_theta_i > 1e-4 && sin_theta_o > 1e-4 {
            let sin_phi_i = sin_phi(wi);
            let cos_phi_i = cos_phi(wi);
            let sin_phi_o = sin_phi(wo);
            let cos_phi_o = cos_phi(wo);
            let d_cos = cos_phi_i * cos_phi_o + sin_phi_i * sin_phi_o;
            d_cos.max(0.0)
        } else {
            0.0
        };

        let (sin_alpha, tan_beta) = if cos_theta(wi).abs() > cos_theta(wo).abs() {
            (sin_theta_o, sin_theta_i / cos_theta(wi).abs())
        } else {
            (sin_theta_i, sin_theta_o / cos_theta(wo).abs())
        };

        self.reflectance
            * std::f32::consts::FRAC_1_PI
            * (self.a + self.b * max_cos * sin_alpha * tan_beta)
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
