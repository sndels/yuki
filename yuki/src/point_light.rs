use crate::{
    hit::Hit,
    math::{point::Point3, transform::Transform, vector::Vec3},
};

/// Sample from a light source for visibility testing and shading
pub struct LightSample {
    pub l: Vec3<f32>,
    pub dist: f32,
    pub li: Vec3<f32>,
}

pub struct PointLight {
    pub p: Point3<f32>,
    pub i: Vec3<f32>,
}

impl PointLight {
    pub fn new(light_to_world: &Transform<f32>, i: Vec3<f32>) -> Self {
        Self {
            p: light_to_world * Point3::new(0.0, 0.0, 0.0),
            i,
        }
    }

    pub fn sample_li(&self, h: &Hit) -> LightSample {
        let to_light = self.p - h.p;
        let dist_sqr = to_light.len_sqr();
        let li = self.i / dist_sqr;
        let dist = dist_sqr.sqrt();
        let l = to_light / dist;

        LightSample { l, dist, li }
    }
}