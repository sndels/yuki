use glium::Surface;
use std::{borrow::Cow, sync::Mutex};
use strum::{EnumString, EnumVariantNames, ToString};

use crate::{
    film::Film,
    math::{Vec2, Vec3},
    yuki_debug, yuki_info, yuki_trace,
};

#[derive(EnumVariantNames, ToString, EnumString)]
pub enum ToneMapType {
    Filmic {
        exposure: f32,
    },
    Heatmap {
        // No bounds forces re-evaluation of tight bounds
        bounds: Option<(f32, f32)>,
        channel: HeatmapChannel,
    },
}

#[derive(Copy, Clone, EnumVariantNames, ToString, EnumString)]
pub enum HeatmapChannel {
    Red = 0,
    Green = 1,
    Blue = 2,
    Luminance = 3,
}

impl Default for HeatmapChannel {
    fn default() -> Self {
        HeatmapChannel::Red
    }
}

pub struct ToneMapFilm {
    vertex_buffer: glium::VertexBuffer<Vertex>,
    index_buffer: glium::IndexBuffer<u16>,
    filmic_program: glium::Program,
    heatmap_program: glium::Program,
    input: glium::Texture2d,
    output: glium::Texture2d,
}

impl ToneMapFilm {
    pub fn new<T: glium::backend::Facade>(backend: &T) -> Result<Self, NewError> {
        let vertex_buffer = glium::VertexBuffer::new(
            backend,
            &[
                Vertex {
                    position: [-3.0, -1.0],
                    uv: [-1.0, 0.0],
                },
                Vertex {
                    position: [1.0, -1.0],
                    uv: [1.0, 0.0],
                },
                Vertex {
                    position: [1.0, 3.0],
                    uv: [1.0, 2.0],
                },
            ],
        )
        .map_err(NewError::VertexBufferCreationError)?;

        let index_buffer = glium::IndexBuffer::new(
            backend,
            glium::index::PrimitiveType::TrianglesList,
            &[0u16, 1, 2],
        )
        .map_err(NewError::IndexBufferCreationError)?;

        let filmic_program = glium::Program::from_source(backend, VS_CODE, FILMIC_FS_CODE, None)
            .map_err(NewError::ProgramCreationError)?;

        let heatmap_program = glium::Program::from_source(backend, VS_CODE, HEATMAP_FS_CODE, None)
            .map_err(NewError::ProgramCreationError)?;

        macro_rules! create_tex {
            () => {
                glium::Texture2d::empty_with_format(
                    backend,
                    FILM_FORMAT,
                    glium::texture::MipmapsOption::NoMipmap,
                    16 as u32,
                    16 as u32,
                )
                .map_err(NewError::TextureCreationError)?
            };
        }
        let input = create_tex!();
        let output = create_tex!();

        Ok(Self {
            vertex_buffer,
            index_buffer,
            filmic_program,
            heatmap_program,
            input,
            output,
        })
    }

    #[must_use]
    pub fn draw<'a, 'b, T: glium::backend::Facade>(
        &'a mut self,
        backend: &T,
        film: &'b Mutex<Film>,
        params: &mut ToneMapType,
    ) -> Result<&'a glium::Texture2d, DrawError<'b>> {
        yuki_trace!("draw: Checking for texture update");
        let film_dirty = self
            .update_textures(backend, film)
            .map_err(DrawError::UpdateTexturesError)?;

        let input_sampler = self
            .input
            .sampled()
            .wrap_function(glium::uniforms::SamplerWrapFunction::BorderClamp)
            .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest)
            .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest);

        match params {
            ToneMapType::Filmic { exposure } => {
                let uniforms = glium::uniform! {
                    input_texture: input_sampler,
                        exposure: *exposure,
                };

                self.output
                    .as_surface()
                    .draw(
                        &self.vertex_buffer,
                        &self.index_buffer,
                        &self.filmic_program,
                        &uniforms,
                        &Default::default(),
                    )
                    .map_err(DrawError::DrawError)?;
            }
            ToneMapType::Heatmap { bounds, channel } => {
                let (min, max) = {
                    if film_dirty || bounds.is_none() {
                        yuki_debug!("draw: Dirty film, re-evaluating heatmap min, max");
                        yuki_trace!("draw: Waiting for lock on film (Heatmap)");
                        let film = film.lock().map_err(DrawError::FilmPoisonError)?;
                        yuki_trace!("draw: Acquired film (Heatmap)");

                        let px_accessor: Box<dyn Fn(Vec3<f32>) -> f32> = match &channel {
                            HeatmapChannel::Red | HeatmapChannel::Green | HeatmapChannel::Blue => {
                                Box::new(|px: Vec3<f32>| px[*channel as usize])
                            }
                            HeatmapChannel::Luminance => Box::new(|px: Vec3<f32>| {
                                0.2126 * px[0] + 0.7152 * px[1] + 0.0722 * px[2]
                            }),
                        };

                        // TODO: This is slow for large films. Do we care?
                        film.pixels()
                            .iter()
                            .fold((f32::MAX, f32::MIN), |(min, max), &px| {
                                let v = px_accessor(px);
                                (min.min(v), max.max(v))
                            })
                    } else {
                        bounds.unwrap()
                    }
                };
                *bounds = Some((min, max));

                let uniforms = glium::uniform! {
                    input_texture: input_sampler,
                    min_val: min,
                    max_val: max,
                    channel: *channel as u32,
                };

                self.output
                    .as_surface()
                    .draw(
                        &self.vertex_buffer,
                        &self.index_buffer,
                        &self.heatmap_program,
                        &uniforms,
                        &Default::default(),
                    )
                    .map_err(DrawError::DrawError)?;
            }
        }

        Ok(&self.output)
    }

    #[must_use]
    fn update_textures<'a, T: glium::backend::Facade>(
        &mut self,
        backend: &T,
        film: &'a Mutex<Film>,
    ) -> Result<bool, UpdateTexturesError<'a>> {
        yuki_trace!("update_film_texture: Begin");
        yuki_trace!("update_film_texture: Waiting for lock on film");
        let mut film = film.lock().map_err(UpdateTexturesError::FilmPoisonError)?;
        yuki_trace!("update_film_texture: Acquired film");

        let film_dirty = film.dirty();
        if film_dirty {
            yuki_debug!("update_film_texture: Film is dirty");
            // We could update only the tiles that have changed but that's more work and scaffolding
            // than it's worth especially with marked tiles. This is fast enough at small resolutions.
            self.input = glium::Texture2d::with_format(
                backend,
                &*film,
                FILM_FORMAT,
                glium::texture::MipmapsOption::NoMipmap,
            )
            .map_err(UpdateTexturesError::TextureCreationError)?;

            if self.input.width() != self.output.width()
                || self.input.height() != self.output.height()
            {
                self.output = glium::Texture2d::empty_with_format(
                    backend,
                    FILM_FORMAT,
                    glium::texture::MipmapsOption::NoMipmap,
                    self.input.width(),
                    self.input.height(),
                )
                .map_err(UpdateTexturesError::TextureCreationError)?
            }

            film.clear_dirty();
            yuki_debug!("update_film_texture: Texture created");
        }

        yuki_trace!("update_film_texture: Releasing film");
        Ok(film_dirty)
    }
}
const FILM_FORMAT: glium::texture::UncompressedFloatFormat =
    glium::texture::UncompressedFloatFormat::F32F32F32;

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
}
glium::implement_vertex!(Vertex, position, uv);

impl<'a> glium::texture::Texture2dDataSource<'a> for &'a Film {
    type Data = Vec3<f32>;

    fn into_raw(self) -> glium::texture::RawImage2d<'a, Vec3<f32>> {
        let Vec2 { x, y } = self.res();
        glium::texture::RawImage2d {
            data: Cow::from(self.pixels()),
            width: x as u32,
            height: y as u32,
            format: glium::texture::ClientFormat::F32F32F32,
        }
    }
}

const VS_CODE: &'static str = r#"
#version 410 core

in vec2 position;
in vec2 uv;

out vec2 frag_uv;

void main() {
    frag_uv = uv;
    gl_Position = vec4(position, 0, 1);
}
"#;

const FILMIC_FS_CODE: &'static str = r#"
#version 410 core

uniform sampler2D input_texture;
uniform float exposure;

in vec2 frag_uv;

out vec4 output_color;

#define saturate(v) clamp(v, 0, 1)

// ACES implementation ported from MJP and David Neubelt's hlsl adaptation of Stephen Hill's fit
// https://github.com/TheRealMJP/BakingLab/blob/master/BakingLab/ACES.hlsl
const mat3 ACESInputMat = transpose(mat3(
    vec3(0.59719f, 0.35458f, 0.04823f),
    vec3(0.07600f, 0.90834f, 0.01566f),
    vec3(0.02840f, 0.13383f, 0.83777f)
));

// ODT_SAT => XYZ => D60_2_D65 => sRGB
const mat3 ACESOutputMat = transpose(mat3(
    vec3( 1.60475f, -0.53108f, -0.07367f),
    vec3(-0.10208f,  1.10813f, -0.00605f),
    vec3(-0.00327f, -0.07276f,  1.07602f)
));

vec3 RRTAndODTFit(vec3 v)
{
    vec3 a = v * (v + 0.0245786f) - 0.000090537f;
    vec3 b = v * (0.983729f * v + 0.4329510f) + 0.238081f;
    return a / b;
}

vec3 ACESFitted(vec3 color)
{
    color = ACESInputMat * color;

    // Apply RRT and ODT
    color = RRTAndODTFit(color);

    color = ACESOutputMat * color;

    // Clamp to [0, 1]
    color = saturate(color);

    return color;
}

void main() {
    vec3 color = texture(input_texture, frag_uv).rgb;
    color *= exposure;
    color = ACESFitted(color);
    output_color = vec4(color, 1.0f);
}
"#;

const HEATMAP_FS_CODE: &'static str = r#"
#version 410 core

uniform sampler2D input_texture;

uniform float min_val;
uniform float max_val;
uniform uint channel;

in vec2 frag_uv;

out vec3 output_color;

#define saturate(v) clamp(v, 0, 1)

const vec3 LOW_COLOR = vec3(0,0,1);
const vec3 MID_COLOR = vec3(0,1,0);
const vec3 HIGH_COLOR = vec3(1,0,0);

void main() {
    float value = 0;
    if (channel > 0 && channel < 3) {
        value = texture(input_texture, frag_uv)[channel];
    } else {
        // Luminance
        value = dot(texture(input_texture, frag_uv).rgb, vec3(0.2126, 0.7152, 0.0722));
    }
    float scaled_value = (value - min_val) / (max_val - min_val);

    // Linear gradient B->G->R
    output_color = mix(
        mix(LOW_COLOR, MID_COLOR, saturate(scaled_value * 2)),
        HIGH_COLOR,
        saturate(scaled_value * 2 - 1));
}
"#;

#[derive(Debug)]
pub enum NewError {
    VertexBufferCreationError(glium::vertex::BufferCreationError),
    IndexBufferCreationError(glium::index::BufferCreationError),
    ProgramCreationError(glium::ProgramCreationError),
    TextureCreationError(glium::texture::TextureCreationError),
}

#[derive(Debug)]
pub enum DrawError<'a> {
    DrawError(glium::DrawError),
    UpdateTexturesError(UpdateTexturesError<'a>),
    FilmPoisonError(std::sync::PoisonError<std::sync::MutexGuard<'a, Film>>),
}

#[derive(Debug)]
pub enum UpdateTexturesError<'a> {
    FilmPoisonError(std::sync::PoisonError<std::sync::MutexGuard<'a, Film>>),
    TextureCreationError(glium::texture::TextureCreationError),
}
