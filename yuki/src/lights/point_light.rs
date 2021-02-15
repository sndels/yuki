use crate::{
    hit::Hit,
    lights::light::{Light, LightSample},
    math::{point::Point3, transform::Transform, vector::Vec3},
};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Light_Sources/Point_Lights.html

pub struct PointLight {
    p: Point3<f32>,
    i: Vec3<f32>,
}

impl PointLight {
    /// Creates a new `PointLight` with the given transform and intensity.
    pub fn new(light_to_world: &Transform<f32>, i: Vec3<f32>) -> Self {
        Self {
            p: light_to_world * Point3::new(0.0, 0.0, 0.0),
            i,
        }
    }
}

impl Light for PointLight {
    fn sample_li(&self, hit: &Hit) -> LightSample {
        let to_light = self.p - hit.p;
        let dist_sqr = to_light.len_sqr();
        let li = self.i / dist_sqr;
        let dist = dist_sqr.sqrt();
        let l = to_light / dist;

        LightSample { l, dist, li }
    }
}
