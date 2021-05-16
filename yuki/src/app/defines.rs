pub type FilmSurface = gfx::format::R32_G32_B32;
pub type FilmFormat = (FilmSurface, gfx::format::Float);
pub type OutputColorFormat = gfx::format::Rgba8;
pub type DepthFormat = gfx::format::DepthStencil;
pub type FilmTextureHandle = gfx::handle::Texture<gfx_device_gl::Resources, FilmSurface>;
