use super::{Light, LightSample};
use crate::{
    interaction::{Interaction, SurfaceInteraction},
    math::{Spectrum, Transform, Vec3},
    visibility::VisibilityTester,
};

// Based on Physically Based Rendering 3rd ed.
// https://pbr-book.org/3ed-2018/Light_Sources/Distant_Lights

pub struct DistantLight {
    w: Vec3<f32>,
    radiance: Spectrum<f32>,
}

impl DistantLight {
    /// Creates a new `DistantLight` with the given transform, radiance and direction.
    pub fn new(radiance: Spectrum<f32>, w: Vec3<f32>) -> Self {
        Self { w, radiance }
    }
}

impl Light for DistantLight {
    fn sample_li(&self, si: &SurfaceInteraction) -> LightSample {
        let li = self.radiance;
        let l = self.w;

        let vis = Some(VisibilityTester::new(
            Interaction::from(si),
            Interaction {
                p: si.p + self.w * 10000.0, // TODO: put point at distance of 2x world radius
                ..Interaction::default()
            },
        ));

        LightSample { l, li, vis }
    }
}
