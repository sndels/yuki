use glium::Surface;
use serde::{Deserialize, Serialize};
use std::{borrow::Cow, sync::Mutex};
use strum::{Display, EnumString, EnumVariantNames};

use crate::{
    film::Film,
    math::{Spectrum, Vec2},
    yuki_debug, yuki_trace,
};

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct FilmicParams {
    pub exposure: f32,
}

impl Default for FilmicParams {
    fn default() -> Self {
        Self { exposure: 1.0 }
    }
}

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct HeatmapParams {
    // No bounds forces re-evaluation of tight bounds
    pub bounds: Option<(f32, f32)>,
    pub channel: HeatmapChannel,
}

impl Default for HeatmapParams {
    fn default() -> Self {
        Self {
            bounds: None,
            channel: HeatmapChannel::Red,
        }
    }
}

#[derive(Copy, Clone, Deserialize, Serialize, Display, EnumVariantNames, EnumString)]
pub enum ToneMapType {
    Raw,
    Filmic(FilmicParams),
    Heatmap(HeatmapParams),
}

#[allow(clippy::derivable_impls)] // Can't derive Default for non unit variants, which Filmic is
impl Default for ToneMapType {
    fn default() -> Self {
        ToneMapType::Filmic(FilmicParams::default())
    }
}

#[derive(Copy, Clone, Deserialize, Serialize, Display, EnumVariantNames, EnumString, PartialEq)]
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
    input_sample_counts: glium::texture::buffer_texture::BufferTexture<f32>,
    tile_dim: u32,
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
        .map_err(NewError::VertexBuffer)?;

        let index_buffer = glium::IndexBuffer::new(
            backend,
            glium::index::PrimitiveType::TrianglesList,
            &[0_u16, 1, 2],
        )
        .map_err(NewError::IndexBuffer)?;

        let filmic_program = glium::Program::from_source(backend, VS_CODE, FILMIC_FS_CODE, None)
            .map_err(NewError::Program)?;

        let heatmap_program = glium::Program::from_source(backend, VS_CODE, HEATMAP_FS_CODE, None)
            .map_err(NewError::Program)?;

        macro_rules! create_tex {
            () => {
                glium::Texture2d::empty_with_format(
                    backend,
                    FILM_FORMAT,
                    glium::texture::MipmapsOption::NoMipmap,
                    16 as u32,
                    16 as u32,
                )
                .map_err(NewError::Texture)?
            };
        }
        let input = create_tex!();
        let output = create_tex!();
        let input_sample_counts = glium::texture::buffer_texture::BufferTexture::empty(
            backend,
            1,
            glium::texture::buffer_texture::BufferTextureType::Float,
        )
        .map_err(NewError::BufferTexture)?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
            filmic_program,
            heatmap_program,
            input,
            input_sample_counts,
            tile_dim: 16,
            output,
        })
    }

    pub fn draw<'a, 'b, T: glium::backend::Facade>(
        &'a mut self,
        backend: &T,
        film: &'b Mutex<Film>,
        params: &ToneMapType,
    ) -> Result<&'a glium::Texture2d, DrawError<'b>> {
        yuki_trace!("draw: Checking for texture update");
        self.update_resources(backend, film)
            .map_err(DrawError::UpdateTextures)?;

        let input_sampler = self
            .input
            .sampled()
            .wrap_function(glium::uniforms::SamplerWrapFunction::BorderClamp)
            .minify_filter(glium::uniforms::MinifySamplerFilter::Nearest)
            .magnify_filter(glium::uniforms::MagnifySamplerFilter::Nearest);

        let output = match params {
            ToneMapType::Raw => &self.input,
            ToneMapType::Filmic(FilmicParams { exposure }) => {
                let uniforms = glium::uniform! {
                    input_texture: input_sampler,
                    input_sample_counts: &self.input_sample_counts,
                    exposure: *exposure,
                    tile_dim: self.tile_dim,
                };

                self.output
                    .as_surface()
                    .draw(
                        &self.vertex_buffer,
                        &self.index_buffer,
                        &self.filmic_program,
                        &uniforms,
                        &glium::DrawParameters::default(),
                    )
                    .map_err(DrawError::Draw)?;

                &self.output
            }
            ToneMapType::Heatmap(HeatmapParams { bounds, channel }) => {
                let (min, max) = bounds.expect("Missing Heatmap bounds");

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
                        &glium::DrawParameters::default(),
                    )
                    .map_err(DrawError::Draw)?;

                &self.output
            }
        };

        Ok(output)
    }

    fn update_resources<'a, T: glium::backend::Facade>(
        &mut self,
        backend: &T,
        film: &'a Mutex<Film>,
    ) -> Result<bool, UpdateResourcesError<'a>> {
        superluminal_perf::begin_event("ToneMapFilm::update_resources");

        yuki_trace!("update_film_texture: Begin");
        yuki_trace!("update_film_texture: Waiting for lock on film");
        let mut film = film.lock().map_err(UpdateResourcesError::FilmPoison)?;
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
            .map_err(UpdateResourcesError::TextureCreation)?;

            self.tile_dim = film.tile_dim().unwrap_or(16) as u32;

            let sample_counts: Vec<f32> = if let Some(samples) = film.samples() {
                samples.iter().map(|s| *s as f32).collect()
            } else {
                let res = film.res();
                // Extra row, column if tiles divide the film unevenly
                let x_tiles = (((res.x as usize) - 1) / (self.tile_dim as usize)) + 1;
                let y_tiles = (((res.y as usize) - 1) / (self.tile_dim as usize)) + 1;
                let tile_count = x_tiles * y_tiles;
                vec![0.0; tile_count]
            };
            self.input_sample_counts = glium::texture::buffer_texture::BufferTexture::new(
                backend,
                sample_counts.as_slice(),
                glium::texture::buffer_texture::BufferTextureType::Float,
            )
            .map_err(UpdateResourcesError::BufferTextureCreation)?;

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
                .map_err(UpdateResourcesError::TextureCreation)?;
            }

            film.clear_dirty();
            yuki_debug!("update_film_texture: Texture created");
        }

        superluminal_perf::end_event(); // ToneMapFilm::update_resources

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
    type Data = Spectrum<f32>;

    fn into_raw(self) -> glium::texture::RawImage2d<'a, Spectrum<f32>> {
        let Vec2 { x, y } = self.res();
        glium::texture::RawImage2d {
            data: Cow::from(self.pixels()),
            width: x as u32,
            height: y as u32,
            format: glium::texture::ClientFormat::F32F32F32,
        }
    }
}

const VS_CODE: &str = r#"
#version 410 core

in vec2 position;
in vec2 uv;

out vec2 frag_uv;

void main() {
    frag_uv = uv;
    gl_Position = vec4(position, 0, 1);
}
"#;

const FILMIC_FS_CODE: &str = r#"
#version 410 core

uniform sampler2D input_texture;
uniform samplerBuffer input_sample_counts;
uniform float exposure;
uniform uint tile_dim;

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

    ivec2 res = textureSize(input_texture, 0);
    int x_tile_count = res.x / int(tile_dim);

    ivec2 tile = ivec2(gl_FragCoord.xy) / int(tile_dim);
    int flat_tile = tile.y * x_tile_count + tile.x; 

    float sample_count = texelFetch(input_sample_counts, flat_tile).x;
    if (sample_count > 0)
        color /= sample_count;
    color *= exposure;
    color = ACESFitted(color);
    output_color = vec4(color, 1.0f);
}
"#;

const HEATMAP_FS_CODE: &str = r#"
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
    VertexBuffer(glium::vertex::BufferCreationError),
    IndexBuffer(glium::index::BufferCreationError),
    Program(glium::ProgramCreationError),
    Texture(glium::texture::TextureCreationError),
    BufferTexture(glium::texture::buffer_texture::CreationError),
}

#[derive(Debug)]
pub enum DrawError<'a> {
    Draw(glium::DrawError),
    UpdateTextures(UpdateResourcesError<'a>),
    FilmPoison(std::sync::PoisonError<std::sync::MutexGuard<'a, Film>>),
}

#[derive(Debug)]
pub enum UpdateResourcesError<'a> {
    FilmPoison(std::sync::PoisonError<std::sync::MutexGuard<'a, Film>>),
    TextureCreation(glium::texture::TextureCreationError),
    BufferTextureCreation(glium::texture::buffer_texture::CreationError),
}

pub fn find_min_max(film: &Mutex<Film>, channel: HeatmapChannel) -> Result<(f32, f32), DrawError> {
    yuki_trace!("find_min_max: Waiting for lock on film");
    let film = film.lock().map_err(DrawError::FilmPoison)?;
    yuki_trace!("find_min_max: Acquired film");

    let px_accessor: Box<dyn Fn(Spectrum<f32>) -> f32> = match &channel {
        HeatmapChannel::Red | HeatmapChannel::Green | HeatmapChannel::Blue => {
            Box::new(|px: Spectrum<f32>| px[channel as usize])
        }
        HeatmapChannel::Luminance => {
            Box::new(|px: Spectrum<f32>| 0.2126 * px.r + 0.7152 * px.g + 0.0722 * px.b)
        }
    };

    // TODO: This is slow for large films. Do we care?
    let ret = film
        .pixels()
        .iter()
        .fold((f32::MAX, f32::MIN), |(min, max), &px| {
            let v = px_accessor(px);
            (min.min(v), max.max(v))
        });

    yuki_trace!("find_min_max: Releasing film");
    Ok(ret)
}
