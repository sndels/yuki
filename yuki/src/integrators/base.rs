use crate::{
    math::{Ray, Vec3},
    scene::Scene,
};

/// Generic interface that needs to be implemented by all Integrators.
pub trait IntegratorBase {
    /// Evaluates the incoming radiance along `ray`. Also returns the number of rays intersected with `scene`.
    fn li(ray: Ray<f32>, scene: &Scene) -> RadianceResult;
}
