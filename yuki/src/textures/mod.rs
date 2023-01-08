mod constant;
mod image_texture;

use crate::interaction::SurfaceInteraction;

pub use constant::ConstantTexture;
pub use image_texture::{ImageTexture, LoadError};

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Texture/Texture_Interface_and_Basic_Textures

pub trait Texture<T>: Send + Sync {
    // TODO: This shouldn't return by value if Spectrum is generalized for larger spectra at some point
    /// Evaluates this `Texture` at the given [`SurfaceInteraction`].
    fn evaluate(&self, si: &SurfaceInteraction) -> T;
}
