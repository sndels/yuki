use crate::{
    error,
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

/// A film tile used for rendering.
pub struct FilmTile {
    /// The [Film] pixel bounds for this tile.
    pub bb: Bounds2<u16>,
    /// Pixel values in this tile, stored in RGB order.
    pub pixels: Vec<Vec<Vec3<f32>>>,
    // Generation of this tile. Used to verify inputs in update_tile.
    generation: u64,
}

impl FilmTile {
    /// Creates a new `FilmTile` with the given [Bounds2].
    pub fn new(bb: Bounds2<u16>, generation: u64) -> Self {
        let width = (bb.p_max.x - bb.p_min.x) as usize;
        let height = (bb.p_max.y - bb.p_min.y) as usize;

        FilmTile {
            bb,
            pixels: vec![vec![Vec3::zeros(); width]; height],
            generation,
        }
    }
}

/// Pixel wrapper for rendering through [FilmTile]s.
pub struct Film {
    // Resolution of the stored pixel buffer.
    res: Vec2<u16>,
    // Pixel values.
    pixels: Vec<Vec3<f32>>,
    // Indicator for changed pixel values.
    dirty: bool,
    // Generation of the storedpixels. Used to verify inputs in update_tile.
    generation: u64,
}

impl Film {
    /// Creates an empty `Film`.
    pub fn default() -> Self {
        Self {
            res: Vec2::new(4, 4),
            pixels: vec![Vec3::zeros(); 4 * 4],
            dirty: true,
            generation: 0,
        }
    }

    /// Returns the resolution of the currently stored pixels of this `Film`.
    pub fn res(&self) -> Vec2<u16> {
        self.res
    }

    /// Returns a reference to the the pixels of this `Film`.
    pub fn pixels(&self) -> &Vec<Vec3<f32>> {
        &self.pixels
    }

    /// Clears the indicator for changed pixel values in this `Film`.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Returns `true` if this `Film`s pixels have been written to since the last
    /// call to its [clear_dirty].
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Resizes this `Film` according to current `settings` and returns [FilmTile]s for rendering.
    /// Pixel values are zeroed.
    /// [FilmTile]s from previous calls should no longer be used.
    pub fn tiles(&mut self, settings: &FilmSettings) -> Vec<FilmTile> {
        // Bump generation for tile verification.
        self.generation += 1;

        // Let's just reallocate each time for brevity
        // TODO: Is there a faster way to pixeldata to 0s?
        self.res = settings.res;
        let pixel_count = (settings.res.x as usize) * (settings.res.y as usize);

        self.pixels = vec![Vec3::zeros(); pixel_count];
        self.dirty = true;

        // Collect tiles spanning the whole image
        let mut tiles = vec![];
        let dim = settings.tile_dim;
        for j in (0..settings.res.y).step_by(dim as usize) {
            for i in (0..settings.res.x).step_by(dim as usize) {
                // Limit tiles to film dimensions
                let max_x = (i + dim).min(settings.res.x);
                let max_y = (j + dim).min(settings.res.y);

                tiles.push(FilmTile::new(
                    Bounds2::new(point2(i, j), point2(max_x, max_y)),
                    self.generation,
                ))
            }
        }

        // TODO: Order tiles in a spiral from middle for more snappy feel when tweaking view

        tiles
    }

    /// Updates this `Film` with the pixel values in a [FilmTile].
    pub fn update_tile(&mut self, tile: &FilmTile) {
        if tile.generation != self.generation {
            error!(format!(
                "Tile generation {} doesn't match film generation {}",
                tile.generation, self.generation
            ));
            return;
        }

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
        for (tile_row, film_row) in ((tile_min.y as usize)..(tile_max.y as usize)).enumerate() {
            let film_row_offset = film_row * (self.res.x as usize);

            let film_slice_start = film_row_offset + (tile_min.x as usize);
            let film_slice_end = film_row_offset + (tile_max.x as usize);

            let film_slice = &mut self.pixels[film_slice_start..film_slice_end];
            let tile_slice = &tile.pixels[tile_row as usize][..];

            film_slice.copy_from_slice(tile_slice);
        }
        self.dirty = true;
    }
}
