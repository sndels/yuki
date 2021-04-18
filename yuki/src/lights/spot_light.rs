use super::{Light, LightSample};
use crate::{
    interaction::SurfaceInteraction,
    math::{Point3, Transform, Vec3},
};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Light_Sources/Point_Lights.html

pub struct SpotLight {
    world_to_light: Transform<f32>,
    p: Point3<f32>,
    i: Vec3<f32>,
    cos_total_width: f32,
    cos_falloff_start: f32,
}

impl SpotLight {
    /// Creates a new `SpotLight` with the given transform, intensity and cone parameters.
    ///
    /// Identity transform has the light pointing down -Z.
    pub fn new(
        light_to_world: &Transform<f32>,
        i: Vec3<f32>,
        total_width_degrees: f32,
        falloff_start_degrees: f32,
    ) -> Self {
        Self {
            world_to_light: light_to_world.inverted(),
            p: light_to_world * Point3::new(0.0, 0.0, 0.0),
            i,
            cos_total_width: total_width_degrees.to_radians().cos(),
            cos_falloff_start: falloff_start_degrees.to_radians().cos(),
        }
    }

    fn falloff(&self, l: Vec3<f32>) -> f32 {
        let dir_local = (&self.world_to_light * -l).normalized();
        let cos_theta = dir_local.z;
        if cos_theta < self.cos_total_width {
            return 0.0;
        }
        if cos_theta > self.cos_falloff_start {
            return 1.0;
        }
        let delta =
            (cos_theta - self.cos_total_width) / (self.cos_falloff_start - self.cos_total_width);
        (delta * delta) * (delta * delta)
    }
}

impl Light for SpotLight {
    fn sample_li(&self, si: &SurfaceInteraction) -> LightSample {
        let to_light = self.p - si.p;
        let dist_sqr = to_light.len_sqr();
        let dist = dist_sqr.sqrt();
        let l = to_light / dist;
        let li = self.i * self.falloff(l) / dist_sqr;

        LightSample { l, dist, li }
    }
}
