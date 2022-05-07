use super::{AreaLight, Light, LightSample};
use crate::{
    interaction::{Interaction, SurfaceInteraction},
    math::{
        transforms::{scale, translation},
        Normal, Point2, Point3, Spectrum, Transform, Vec2, Vec3,
    },
    visibility::VisibilityTester,
};

use approx::relative_eq;

// Based on Physically Based Rendering 3rd ed.
// https://pbr-book.org/3ed-2018/Light_Sources/Area_Lights
// https://www.pbr-book.org/3ed-2018/Light_Transport_I_Surface_Reflection/Sampling_Light_Sources

/// Rectangular light
/// Identity transform facing -y at origin
pub struct RectangularLight {
    sample_to_world: Transform<f32>,
    l: Spectrum<f32>,
    area: f32,
}

impl RectangularLight {
    /// Creates a new `RectangularLight` with the given transform, radiance and size in meters.
    pub fn new(light_to_world: &Transform<f32>, l: Spectrum<f32>, size: Vec2<f32>) -> Self {
        assert!(
            relative_eq!(light_to_world.m().decompose().unwrap().scale, Vec3::ones()),
            "Light transform should have no scaling!"
        );
        // Samples we get are [0, 1)
        let sample_to_light =
            &scale(size.x, 1.0, size.y) * &translation(Vec3::new(-0.5, 0.0, -0.5));
        let sample_to_world = light_to_world * &sample_to_light;
        let area = size.x * size.y;
        Self {
            sample_to_world,
            l,
            area,
        }
    }
}

impl Light for RectangularLight {
    fn sample_li(&self, si: &SurfaceInteraction, u: Point2<f32>) -> LightSample {
        let p = &self.sample_to_world * Point3::new(u.x, 0.0, u.y);
        let n = &self.sample_to_world * Normal::new(0.0, -1.0, 0.0);

        let wi = (p - si.p).normalized();
        let li = if n.dot_v(-wi) > 0.0 {
            self.l
        } else {
            Spectrum::zeros()
        };

        let vis = Some(VisibilityTester::new(
            Interaction::from(si),
            Interaction { p, n },
        ));

        let pdf = si.p.dist_sqr(p) / (n.dot_v(-wi).abs() * self.area);

        LightSample {
            l: wi,
            li,
            vis,
            pdf,
        }
    }
}

impl AreaLight for RectangularLight {
    fn radiance(&self, si: &SurfaceInteraction, w: Vec3<f32>) -> Spectrum<f32> {
        if si.n.dot_v(w) > 0.0 {
            self.l
        } else {
            Spectrum::zeros()
        }
    }
}
