use super::Texture;
use crate::{interaction::SurfaceInteraction, math::Spectrum};

use image::io::Reader as ImageReader;
use std::{io::Cursor, path::PathBuf};

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Texture/Image_Texture

macro_rules! load_u8_spectrum_image {
    ($data:expr, $ib:expr) => {
        for row in $ib.rows() {
            for p in row {
                $data.push(Spectrum::new(
                    (p[0] as f32) / (std::u8::MAX as f32),
                    (p[1] as f32) / (std::u8::MAX as f32),
                    (p[2] as f32) / (std::u8::MAX as f32),
                ));
            }
        }
    };
}

macro_rules! load_u16_spectrum_image {
    ($data:expr, $ib:expr) => {
        for row in $ib.rows() {
            for p in row {
                $data.push(Spectrum::new(
                    (p[0] as f32) / (std::u16::MAX as f32),
                    (p[1] as f32) / (std::u16::MAX as f32),
                    (p[2] as f32) / (std::u16::MAX as f32),
                ));
            }
        }
    };
}

macro_rules! load_f32_spectrum_image {
    ($data:expr, $ib:expr) => {
        for row in $ib.rows() {
            for p in row {
                $data.push(Spectrum::new(p[0], p[1], p[2]));
            }
        }
    };
}

pub struct ImageTexture<T>
where
    T: Copy + Send + Sync,
{
    data: Vec<T>,
    width: usize,
    height: usize,
}

#[derive(Debug)]
pub enum LoadError {
    IoError(std::io::Error),
    DecodeError(image::error::ImageError),
    FormatError(String),
}

// TODO: impl for T, check that input file matches
impl ImageTexture<Spectrum<f32>> {
    pub fn new(path: &PathBuf) -> Result<Self, LoadError> {
        let img = ImageReader::open(path).map_err(LoadError::IoError)?;

        load_image_spectrum_f32(img)
    }

    pub fn from_image_bytes(bytes: &[u8]) -> Result<Self, LoadError> {
        let img = ImageReader::new(Cursor::new(bytes))
            .with_guessed_format()
            .map_err(LoadError::IoError)?;

        load_image_spectrum_f32(img)
    }
}

impl<T> Texture<T> for ImageTexture<T>
where
    T: Copy + Send + Sync,
{
    fn evaluate(&self, si: &SurfaceInteraction) -> T {
        // TODO: Mapping (UVMapping2D with scale and offset)
        let mut st = si.uv;

        // Repeat
        st.x = st.x.fract();
        if st.x < 0.0 {
            st.x = 1.0 + st.x;
        }
        st.y = st.y.fract();
        if st.y < 0.0 {
            st.y = 1.0 + st.y;
        }

        // Flip y
        st.y = 1.0 - st.y;

        // TODO: Split into MipMap like in pbrt
        {
            let mut st = st;
            st.x = st.x * (self.width as f32) - 0.5;
            st.y = st.y * (self.height as f32) - 0.5;

            self.data[(st.y as usize) * self.width + (st.x as usize)]
        }
    }
}

fn load_image_spectrum_f32<R: std::io::Read + std::io::BufRead + std::io::Seek>(
    img_reader: ImageReader<R>,
) -> Result<ImageTexture<Spectrum<f32>>, LoadError> {
    let img = img_reader.decode().map_err(LoadError::DecodeError)?;

    let width = img.width() as usize;
    let height = img.height() as usize;

    let mut data = Vec::with_capacity(width * height);
    match img {
        image::DynamicImage::ImageRgb8(ib) => load_u8_spectrum_image!(data, &ib),
        image::DynamicImage::ImageRgba8(ib) => load_u8_spectrum_image!(data, &ib),
        image::DynamicImage::ImageRgb16(ib) => load_u16_spectrum_image!(data, ib),
        image::DynamicImage::ImageRgba16(ib) => load_u16_spectrum_image!(data, ib),
        image::DynamicImage::ImageRgb32F(ib) => load_f32_spectrum_image!(data, ib),
        image::DynamicImage::ImageRgba32F(ib) => load_f32_spectrum_image!(data, ib),
        _ => {
            return Err(LoadError::FormatError(
                "Unsupported image format".to_string(),
            ))
        }
    }

    Ok(ImageTexture {
        data,
        width,
        height,
    })
}
