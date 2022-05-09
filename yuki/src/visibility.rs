use crate::{interaction::Interaction, lights::AreaLight, math::Ray, scene::Scene};

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Light_Sources/Light_Interface#VisibilityTesting

pub struct VisibilityTester<'a> {
    p0: Interaction,
    p1: Interaction,
    area_light: Option<&'a dyn AreaLight>,
}

impl<'a> VisibilityTester<'a> {
    pub fn new(p0: Interaction, p1: Interaction, area_light: Option<&'a dyn AreaLight>) -> Self {
        Self { p0, p1, area_light }
    }

    pub fn ray(&self) -> Ray<f32> {
        self.p0.spawn_ray_to(&self.p1)
    }

    pub fn unoccluded(&self, scene: &Scene) -> bool {
        !scene.bvh.any_intersect(self.ray(), self.area_light)
    }
}
