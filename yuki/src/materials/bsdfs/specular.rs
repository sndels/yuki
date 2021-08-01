use super::{cos_theta, fresnel, fresnel::Fresnel, refract, BxDF, BxdfSample, BxdfType};
use crate::math::{Normal, Vec3};

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Reflection_Models/Specular_Reflection_and_Transmission

pub struct Reflection {
    r: Vec3<f32>,
    fresnel: Box<dyn Fresnel>,
}

impl Reflection {
    pub fn new(r: Vec3<f32>, fresnel: Box<dyn Fresnel>) -> Self {
        Self { r, fresnel }
    }
}

impl BxDF for Reflection {
    fn f(&self, _: Vec3<f32>, _: Vec3<f32>) -> Vec3<f32> {
        Vec3::from(0.0)
    }

    fn sample_f(&self, wo: Vec3<f32>) -> BxdfSample {
        // TODO: Do color/spectrum class for this math
        fn mul(v1: Vec3<f32>, v2: Vec3<f32>) -> Vec3<f32> {
            Vec3::new(v1.x * v2.x, v1.y * v2.y, v1.z * v2.z)
        }

        // Bsdf n = (0,0,1)
        let wi = Vec3::new(-wo.x, -wo.y, wo.z);
        let f = mul(self.r, self.fresnel.evaluate(cos_theta(wi))) / cos_theta(wi).abs();
        BxdfSample {
            wi,
            f,
            sample_type: self.flags(),
        }
    }

    fn flags(&self) -> BxdfType {
        BxdfType::SPECULAR | BxdfType::REFLECTION
    }
}

pub struct Transmission {
    t: Vec3<f32>,
    eta_i: f32,
    eta_t: f32,
    fresnel: fresnel::Dielectric,
}

impl Transmission {
    pub fn new(t: Vec3<f32>, eta_i: f32, eta_t: f32) -> Self {
        Self {
            t,
            eta_i,
            eta_t,
            fresnel: fresnel::Dielectric::new(eta_i, eta_t),
        }
    }
}

impl BxDF for Transmission {
    fn f(&self, _: Vec3<f32>, _: Vec3<f32>) -> Vec3<f32> {
        Vec3::from(0.0)
    }

    fn sample_f(&self, wo: Vec3<f32>) -> BxdfSample {
        // TODO: Do color/spectrum class for this math
        fn mul(v1: Vec3<f32>, v2: Vec3<f32>) -> Vec3<f32> {
            Vec3::new(v1.x * v2.x, v1.y * v2.y, v1.z * v2.z)
        }

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
        .map_or_else(
            || BxdfSample {
                wi: Vec3::from(0.0),
                f: Vec3::from(0.0),
                sample_type: BxdfType::NONE,
            },
            |wi| {
                let f = mul(
                    self.t,
                    Vec3::from(1.0) - self.fresnel.evaluate(cos_theta(wi)),
                ) / cos_theta(wi).abs();
                BxdfSample {
                    wi,
                    f,
                    sample_type: self.flags(),
                }
            },
        )
    }

    fn flags(&self) -> BxdfType {
        BxdfType::SPECULAR | BxdfType::TRANSMISSION
    }
}
