use super::Texture;
use crate::interaction::SurfaceInteraction;

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Texture/Texture_Interface_and_Basic_Textures

pub struct ConstantTexture<T>
where
    T: Copy + Send + Sync,
{
    value: T,
}

impl<T> ConstantTexture<T>
where
    T: Copy + Send + Sync,
{
    pub fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T> Texture<T> for ConstantTexture<T>
where
    T: Copy + Send + Sync,
{
    fn evaluate(&self, _si: &SurfaceInteraction) -> T {
        self.value
    }
}
