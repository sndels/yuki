use crate::{interaction::Interaction, math::Ray, scene::Scene};

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Light_Sources/Light_Interface#VisibilityTesting

pub struct VisibilityTester {
    p0: Interaction,
    p1: Interaction,
}

impl VisibilityTester {
    pub fn new(p0: Interaction, p1: Interaction) -> Self {
        Self { p0, p1 }
    }

    pub fn ray(&self) -> Ray<f32> {
        self.p0.spawn_ray_to(&self.p1)
    }

    pub fn unoccluded(&self, scene: &Scene) -> bool {
        !scene.bvh.any_intersect(self.ray())
    }
}
