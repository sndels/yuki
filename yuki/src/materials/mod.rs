mod bsdfs;
mod matte;

pub use bsdfs::BSDF;
pub use matte::Matte;

use crate::interaction::SurfaceInteraction;

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Materials/Material_Interface_and_Implementations

pub trait Material: Send + Sync {
    /// Returns the [BSDF] for the given [SurfaceInteraction]
    fn compute_scattering_functions(&self, si: &SurfaceInteraction) -> BSDF;
}