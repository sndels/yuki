use crate::math::Spectrum;

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Reflection_Models/Specular_Reflection_and_Transmission

pub trait Fresnel {
    fn evaluate(&self, cos_theta_i: f32) -> Spectrum<f32>;
}

pub struct Dielectric {
    eta_i: f32,
    eta_t: f32,
}

impl Dielectric {
    pub fn new(eta_i: f32, eta_t: f32) -> Self {
        Self { eta_i, eta_t }
    }
}

impl Fresnel for Dielectric {
    fn evaluate(&self, mut cos_theta_i: f32) -> Spectrum<f32> {
        cos_theta_i = cos_theta_i.clamp(-1.0, 1.0);

        let entering = cos_theta_i > 0.0;
        let (eta_i, eta_t, cos_theta_i) = if entering {
            (self.eta_i, self.eta_t, cos_theta_i)
        } else {
            (self.eta_t, self.eta_i, cos_theta_i.abs())
        };

        // Snell's law
        let sin_theta_i = (1.0 - cos_theta_i * cos_theta_i).max(0.0).sqrt();
        let sin_theta_t = eta_i / eta_t * sin_theta_i;

        let total_internal_reflection = sin_theta_t >= 1.0;
        if total_internal_reflection {
            return Spectrum::ones();
        }

        let cos_theta_t = (1.0 - sin_theta_t * sin_theta_t).max(0.0).sqrt();

        let r_parallel = ((eta_t * cos_theta_i) - (eta_i * cos_theta_t))
            / ((eta_t * cos_theta_i) + (eta_i * cos_theta_t));
        let r_perpendicular = ((eta_i * cos_theta_i) - (eta_t * cos_theta_t))
            / ((eta_i * cos_theta_i) + (eta_t * cos_theta_t));

        Spectrum::ones() * (r_parallel * r_parallel + r_perpendicular * r_perpendicular) / 2.0
    }
}
