use std::{ops::DerefMut, sync::Mutex};

use crate::{
    error, expect,
    math::{
        bounds::Bounds2,
        point::point2,
        vector::{Vec2, Vec3},
    },
};

/// The settings for a `Film`.
#[derive(Debug, Copy, Clone)]
pub struct FilmSettings {
    /// The total film resolution.
    pub res: Vec2<u16>,
    /// The tile size to be used.
    pub tile_dim: u16,
}

impl FilmSettings {
    /// Creates a new `FilmSettings` with `res` [640, 480] and `tile_dim` [8,8].
    pub fn default() -> Self {
        Self {
            res: Vec2::new(640, 480),
            tile_dim: 16,
        }
    }
}

/// A film tile used for asynchronous rendering.
pub struct FilmTile {
    /// The [Film] pixel bounds for this tile.
    pub bb: Bounds2<u16>,
    /// Pixel values in this tile, stored in RGB order.
    pub pixels: Vec<Vec<Vec3<f32>>>,
}

impl FilmTile {
    /// Creates a new `FilmTile` with the given [Bounds2].
    pub fn new(bb: Bounds2<u16>) -> Self {
        let width = (bb.p_max.x - bb.p_min.x) as usize;
        let height = (bb.p_max.y - bb.p_min.y) as usize;

        FilmTile {
            bb,
            pixels: vec![vec![Vec3::zeros(); width]; height],
        }
    }
}

/// Pixel data of a [Film].
pub struct FilmPixels {
    // Raw pixels values.
    pub pixels: Vec<Vec3<f32>>,
    // TODO: Accumulation store sample counts per pixel
    // Set to `true` when the [Film] updates the pixels.
    pub dirty: bool,
}

/// Pixel wrapper for asynchronous rendering through [FilmTile]s.
pub struct Film {
    // Resolution of the stored pixel buffer.
    res: Vec2<u16>,
    // Pixels wrapped in a mutex for parallel tile rendering.
    pixels: Mutex<FilmPixels>,
}

impl Film {
    /// Creates an empty `Film`.
    pub fn default() -> Self {
        Self {
            res: Vec2::new(4, 4),
            pixels: Mutex::new(FilmPixels {
                pixels: vec![Vec3::zeros(); 4 * 4],
                dirty: true,
            }),
        }
    }

    /// Clears this `Film` with `col`.
    pub fn clear(&mut self, col: Vec3<f32>) {
        let mut pixel_lock = expect!(self.pixels.lock(), "Failed to acquire lock on film pixels");

        let FilmPixels {
            ref mut pixels,
            ref mut dirty,
        } = pixel_lock.deref_mut();

        pixels.iter_mut().for_each(|v| *v = col);

        *dirty = true;
    }

    /// Resizes this `Film` according to current `settings` and returns [FilmTile]s for rendering.
    /// Caller should make sure that the previous tiles are no longer in use.
    pub fn tiles(&mut self, settings: &FilmSettings) -> Vec<FilmTile> {
        // Resize pixel storage
        if settings.res != self.res {
            let pixel_count = (settings.res.x as usize) * (settings.res.y as usize);
            let mut pixel_lock =
                expect!(self.pixels.lock(), "Failed to acquire lock on film pixels");
            let FilmPixels {
                ref mut pixels,
                ref mut dirty,
            } = pixel_lock.deref_mut();

            *pixels = vec![Vec3::zeros(); pixel_count];

            self.res = settings.res;
            *dirty = true;
        }

        // Collect tiles spanning the whole image
        let mut tiles = vec![];
        let dim = settings.tile_dim;
        for j in (0..settings.res.y).step_by(dim as usize) {
            for i in (0..settings.res.x).step_by(dim as usize) {
                // Limit tiles to film dimensions
                let max_x = (i + dim).min(settings.res.x);
                let max_y = (j + dim).min(settings.res.y);

                tiles.push(FilmTile::new(Bounds2::new(
                    point2(i, j),
                    point2(max_x, max_y),
                )))
            }
        }

        // TODO: Order tiles in a spiral from middle for more snappy feel when tweaking view

        tiles
    }

    /// Updates this `Film` with the pixel values in a [FilmTile].
    pub fn update_tile(&mut self, tile: &FilmTile) {
        let tile_min = tile.bb.p_min;
        let tile_max = tile.bb.p_max;

        if tile_max.x > self.res.x || tile_max.y > self.res.y {
            error!(format!(
                "Tile doesn't fit film ({:?} {:?})",
                self.res, tile.bb
            ));
            return;
        }

        // Copy pixels over to the film
        // TODO: Accumulation, store counts per pixel
        let mut pixel_lock = expect!(self.pixels.lock(), "Failed to acquire lock on film pixels");
        let FilmPixels {
            ref mut pixels,
            ref mut dirty,
        } = pixel_lock.deref_mut();
        for (tile_row, film_row) in ((tile_min.y as usize)..(tile_max.y as usize)).enumerate() {
            let film_row_offset = film_row * (self.res.x as usize);

            let film_slice_start = film_row_offset + (tile_min.x as usize);
            let film_slice_end = film_row_offset + (tile_max.x as usize);

            let film_slice = &mut pixels[film_slice_start..film_slice_end];
            let tile_slice = &tile.pixels[tile_row as usize][..];

            film_slice.copy_from_slice(tile_slice);
        }
        *dirty = true;
    }

    /// Returns the resolution of the currently stored pixels of this `Film`.
    pub fn res(&self) -> Vec2<u16> {
        self.res
    }

    /// Returns a mutable reference to the the pixels of this `Film`.
    pub fn pixels(&mut self) -> &mut Mutex<FilmPixels> {
        &mut self.pixels
    }
}
