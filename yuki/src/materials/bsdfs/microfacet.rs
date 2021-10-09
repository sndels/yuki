use super::{cos_theta, fresnel::Fresnel, reflect, same_hemisphere, Bxdf, BxdfSample, BxdfType};
use crate::math::{Normal, Point2, Spectrum, Vec3};

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Reflection_Models/Microfacet_Models#MicrofacetDistributionFunctions
// https://www.pbr-book.org/3ed-2018/Reflection_Models/Microfacet_Models#MaskingandShadowing
// https://www.pbr-book.org/3ed-2018/Reflection_Models/Microfacet_Models#TheTorrancendashSparrowModel

pub trait MicrofacetDistribution {
    /// Evaluates the distribution function for the given surface normal
    fn d(&self, wh: Vec3<f32>) -> f32;
    /// Evaluates the invisible masked microfacet area per visible microfacet area
    fn lambda(&self, w: Vec3<f32>) -> f32;
    /// Samples the normal distribution for the given view direction
    fn sample_wh(&self, wo: Vec3<f32>, u: Point2<f32>) -> Vec3<f32>;
    /// Evaluates the pdf for the given view direction and surface normal
    fn pdf(&self, wo: Vec3<f32>, wh: Vec3<f32>) -> f32;

    /// Evaluate Smith's masking-shadowing function for the given direction
    fn g1(&self, w: Vec3<f32>) -> f32 {
        1.0 / (1.0 + self.lambda(w))
    }

    /// Evaluate the fraction of microfacets that are visible from both wo and wi
    fn g(&self, wo: Vec3<f32>, wi: Vec3<f32>) -> f32 {
        1.0 / (1.0 + self.lambda(wo) + self.lambda(wi))
    }
}

pub struct MicrofacetReflection {
    r: Spectrum<f32>,
    distribution: Box<dyn MicrofacetDistribution>,
    fresnel: Box<dyn Fresnel>,
}

impl MicrofacetReflection {
    pub fn new(
        r: Spectrum<f32>,
        distribution: Box<dyn MicrofacetDistribution>,
        fresnel: Box<dyn Fresnel>,
    ) -> Self {
        Self {
            r,
            distribution,
            fresnel,
        }
    }
}

impl Bxdf for MicrofacetReflection {
    fn f(&self, wo: Vec3<f32>, wi: Vec3<f32>) -> Spectrum<f32> {
        let cos_theta_o = cos_theta(wo).abs();
        let cos_theta_i = cos_theta(wi).abs();
        if cos_theta_i == 0.0 || cos_theta_o == 0.0 {
            return Spectrum::zeros();
        }

        let wh = {
            let wh = wi + wo;
            if wh == Vec3::zeros() {
                return Spectrum::zeros();
            }
            wh.normalized()
        };
        let f = self.fresnel.evaluate(wi.dot(Vec3::from(
            Normal::from(wh).faceforward_v(Vec3::new(0.0, 0.0, 1.0)),
        )));

        self.r * self.distribution.d(wh) * self.distribution.g(wo, wi) * f
            / (4.0 * cos_theta_i * cos_theta_o)
    }

    fn sample_f(&self, wo: Vec3<f32>, u: Point2<f32>) -> BxdfSample {
        // Bsdf n = (0,0,1)
        if wo.z == 0.0 {
            return BxdfSample::default();
        }

        let wh = self.distribution.sample_wh(wo, u);
        if wo.dot(wh) < 0.0 {
            return BxdfSample::default();
        }

        let wi = reflect(wo, wh);
        if !same_hemisphere(wo, wi) {
            return BxdfSample::default();
        }

        let pdf = self.distribution.pdf(wo, wh) / (4.0 * wo.dot(wh));

        let f = self.f(wo, wi);

        BxdfSample {
            wi,
            f,
            pdf,
            sample_type: self.flags(),
        }
    }

    fn pdf(&self, wo: Vec3<f32>, wi: Vec3<f32>) -> f32 {
        if !same_hemisphere(wo, wi) {
            return 0.0;
        }
        let wh = (wo + wi).normalized();

        self.distribution.pdf(wo, wh) / (4.0 * wo.dot(wh))
    }

    fn flags(&self) -> BxdfType {
        BxdfType::REFLECTION | BxdfType::GLOSSY
    }
}
