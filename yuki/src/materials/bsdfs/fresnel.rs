use crate::math::Spectrum;

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Reflection_Models/Specular_Reflection_and_Transmission
// https://www.pbr-book.org/3ed-2018/Reflection_Models/Fresnel_Incidence_Effects

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

pub struct Conductor {
    eta_i: Spectrum<f32>,
    eta_t: Spectrum<f32>,
    k: Spectrum<f32>,
}

impl Conductor {
    pub fn new(eta_i: Spectrum<f32>, eta_t: Spectrum<f32>, k: Spectrum<f32>) -> Self {
        Self { eta_i, eta_t, k }
    }
}

impl Fresnel for Conductor {
    fn evaluate(&self, mut cos_theta_i: f32) -> Spectrum<f32> {
        // TODO: Move to Specturm if this is needed elsewhere
        fn sqrt(v: Spectrum<f32>) -> Spectrum<f32> {
            Spectrum::new(v[0].sqrt(), v[1].sqrt(), v[2].sqrt())
        }

        // pbrt does the abs before calling a helper that does the clamp into [-1,1]
        cos_theta_i = cos_theta_i.abs().min(1.0);
        let eta = self.eta_t / self.eta_i;
        let eta_k = self.k / self.eta_i;

        let cos_theta_i_2 = cos_theta_i * cos_theta_i;
        let sin_theta_i_2 = 1.0 - cos_theta_i_2;
        let eta_2 = eta * eta;
        let eta_k_2 = eta_k * eta_k;

        let t0 = eta_2 - eta_k_2 - sin_theta_i_2;
        let a_2_plus_b_2 = sqrt(t0 * t0 + eta_2 * eta_k_2 * 4.0);
        let t1 = a_2_plus_b_2 + cos_theta_i_2;
        let a = sqrt((a_2_plus_b_2 + t0) * 0.5);
        let t2 = a * cos_theta_i * 2.0;
        let rs = (t1 - t2) / (t1 + t2);

        let t3 = a_2_plus_b_2 * cos_theta_i_2 + sin_theta_i_2 * sin_theta_i_2;
        let t4 = t2 * sin_theta_i_2;
        let rp = rs * (t3 - t4) / (t3 + t4);

        (rp + rs) * 0.5
    }
}

pub struct Schlick {
    rs: Spectrum<f32>,
}

impl Schlick {
    pub fn new(rs: Spectrum<f32>) -> Self {
        Self { rs }
    }
}

impl Fresnel for Schlick {
    fn evaluate(&self, mut cos_theta_i: f32) -> Spectrum<f32> {
        fn pow5(v: f32) -> f32 {
            (v * v) * (v * v) * v
        }

        cos_theta_i = cos_theta_i.clamp(-1.0, 1.0);

        self.rs + (Spectrum::ones() - self.rs) * pow5(1.0 - cos_theta_i)
    }
}
