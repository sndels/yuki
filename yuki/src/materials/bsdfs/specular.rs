use super::{cos_theta, fresnel, fresnel::Fresnel, refract, Bxdf, BxdfSample, BxdfType};
use crate::math::{Normal, Point2, Spectrum, Vec3};

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Reflection_Models/Specular_Reflection_and_Transmission

pub struct Reflection {
    r: Spectrum<f32>,
    fresnel: Box<dyn Fresnel>,
}

impl Reflection {
    pub fn new(r: Spectrum<f32>, fresnel: Box<dyn Fresnel>) -> Self {
        Self { r, fresnel }
    }
}

impl Bxdf for Reflection {
    fn f(&self, _: Vec3<f32>, _: Vec3<f32>) -> Spectrum<f32> {
        Spectrum::zeros()
    }

    fn sample_f(&self, wo: Vec3<f32>, _u: Point2<f32>) -> BxdfSample {
        // Bsdf n = (0,0,1)
        let wi = Vec3::new(-wo.x, -wo.y, wo.z);
        let f = self.r * self.fresnel.evaluate(cos_theta(wi)) / cos_theta(wi).abs();
        BxdfSample {
            wi,
            f,
            pdf: 1.0,
            sample_type: self.flags(),
        }
    }

    fn pdf(&self, _w0: Vec3<f32>, _wi: Vec3<f32>) -> f32 {
        1.0
    }

    fn flags(&self) -> BxdfType {
        BxdfType::SPECULAR | BxdfType::REFLECTION
    }
}

pub struct Transmission {
    t: Spectrum<f32>,
    eta_i: f32,
    eta_t: f32,
    fresnel: fresnel::Dielectric,
}

impl Transmission {
    pub fn new(t: Spectrum<f32>, eta_i: f32, eta_t: f32) -> Self {
        Self {
            t,
            eta_i,
            eta_t,
            fresnel: fresnel::Dielectric::new(eta_i, eta_t),
        }
    }
}

impl Bxdf for Transmission {
    fn f(&self, _: Vec3<f32>, _: Vec3<f32>) -> Spectrum<f32> {
        Spectrum::zeros()
    }

    fn sample_f(&self, wo: Vec3<f32>, _u: Point2<f32>) -> BxdfSample {
        let entering = cos_theta(wo) > 0.0;
        let (eta_i, eta_t) = if entering {
            (self.eta_i, self.eta_t)
        } else {
            (self.eta_t, self.eta_i)
        };

        // Bsdf n = (0,0,1)
        refract(
            wo,
            Normal::new(0.0, 0.0, 1.0).faceforward(wo),
            eta_i / eta_t,
        )
        .map_or_else(BxdfSample::default, |wi| {
            let f = self.t * (Spectrum::ones() - self.fresnel.evaluate(cos_theta(wi)))
                / cos_theta(wi).abs();
            BxdfSample {
                wi,
                f,
                pdf: 1.0,
                sample_type: self.flags(),
            }
        })
    }

    fn pdf(&self, _w0: Vec3<f32>, _wi: Vec3<f32>) -> f32 {
        1.0
    }

    fn flags(&self) -> BxdfType {
        BxdfType::SPECULAR | BxdfType::TRANSMISSION
    }
}
