mod bsdfs;
mod glass;
mod glossy;
mod matte;
mod metal;

pub use bsdfs::{Bsdf, BxdfSample, BxdfType};
pub use glass::Glass;
pub use glossy::Glossy;
pub use matte::Matte;
pub use metal::Metal;

use allocators::ScopedScratch;

use crate::interaction::SurfaceInteraction;

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Materials/Material_Interface_and_Implementations

pub trait Material: Send + Sync {
    /// Returns the [`Bsdf`] for the given [`SurfaceInteraction`]
    fn compute_scattering_functions<'a>(
        &self,
        scratch: &'a ScopedScratch,
        si: &SurfaceInteraction,
    ) -> Bsdf<'a>;
}
