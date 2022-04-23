use super::{
    cos_2_phi, cos_2_theta, cos_theta, microfacet::MicrofacetDistribution, same_hemisphere,
    sin_2_phi, spherical_direction, tan_2_theta, tan_theta,
};
use crate::math::{Point2, Vec3};

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Reflection_Models/Microfacet_Models#MicrofacetDistributionFunctions
// https://www.pbr-book.org/3ed-2018/Reflection_Models/Microfacet_Models#MaskingandShadowing

pub struct TrowbridgeReitzDistribution {
    alpha: f32,
}

impl TrowbridgeReitzDistribution {
    pub fn new(alpha: f32) -> Self {
        Self {
            alpha: alpha.max(0.001),
        }
    }

    #[allow(clippy::excessive_precision)] // In case f64 is used at some point
    pub fn roughness_to_alpha(roughness: f32) -> f32 {
        let x = roughness.max(0.001).ln();
        1.621_42
            + 0.819_955 * x
            + 0.173_4 * x * x
            + 0.017_120_1 * x * x * x
            + 0.000_640_711 * x * x * x * x
    }
}

impl MicrofacetDistribution for TrowbridgeReitzDistribution {
    fn d(&self, wh: Vec3<f32>) -> f32 {
        let tan_2_theta = tan_2_theta(wh);
        if tan_2_theta.is_infinite() {
            return 0.0;
        }

        let alpha_2 = self.alpha * self.alpha;
        let cos_4_theta = cos_2_theta(wh) * cos_2_theta(wh);
        let e = (cos_2_phi(wh) / alpha_2 + sin_2_phi(wh) / alpha_2) * tan_2_theta;
        1.0 / (std::f32::consts::PI * alpha_2 * cos_4_theta * (1.0 + e) * (1.0 + e))
    }

    fn lambda(&self, w: Vec3<f32>) -> f32 {
        let abs_tan_theta = tan_theta(w).abs();
        if abs_tan_theta.is_infinite() {
            return 0.0;
        }

        let alpha = (cos_2_phi(w) * self.alpha * self.alpha
            + sin_2_phi(w) * self.alpha * self.alpha)
            .sqrt();

        let alpha_2_tan_2_theta = (alpha * abs_tan_theta) * (alpha * abs_tan_theta);
        (-1.0 + (1.0 + alpha_2_tan_2_theta).sqrt()) / 2.0
    }

    fn sample_wh(&self, wo: Vec3<f32>, u: Point2<f32>) -> Vec3<f32> {
        // TODO: Visible area sampling. Bench the difference
        let tan_theta_2 = self.alpha * self.alpha * u[0] / (1.0 - u[0]);
        let cos_theta = 1.0 / (1.0 + tan_theta_2).sqrt();
        let phi = 2.0 * std::f32::consts::PI * u[1];

        let sin_theta = (1.0 - cos_theta * cos_theta).max(0.0).sqrt();

        let wh = spherical_direction(sin_theta, cos_theta, phi);
        if same_hemisphere(wo, wh) {
            wh
        } else {
            -wh
        }
    }

    fn pdf(&self, _wo: Vec3<f32>, wh: Vec3<f32>) -> f32 {
        self.d(wh) * cos_theta(wh)
    }
}
