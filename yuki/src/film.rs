use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

use crate::{
    math::{
        bounds::Bounds2,
        point::Point2,
        vector::{Vec2, Vec3},
    },
    yuki_debug, yuki_error, yuki_trace, yuki_warn,
};

/// The settings for a `Film`.
#[derive(Debug, Copy, Clone)]
pub struct FilmSettings {
    /// The total film resolution.
    pub res: Vec2<u16>,
    /// The tile size to be used.
    pub tile_dim: u16,
    /// `true` if pixels need to be cleared even if the buffer is not resized
    pub clear: bool,
    /// Value to clear the buffer with
    pub clear_color: Vec3<f32>,
}

impl FilmSettings {
    /// Creates a new `FilmSettings` with `res` [640, 480], `tile_dim` [8,8] and clearing on with black color.
    pub fn default() -> Self {
        Self {
            res: Vec2::new(640, 480),
            tile_dim: 16,
            clear: true,
            clear_color: Vec3::zeros(),
        }
    }
}

/// A film tile used for rendering.
#[derive(Debug, Clone)]
pub struct FilmTile {
    /// The [Film] pixel bounds for this tile.
    pub bb: Bounds2<u16>,
    /// Pixel values in this tile stored in row-major RGB order.
    pub pixels: Vec<Vec3<f32>>,
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
            pixels: vec![Vec3::zeros(); width * height],
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
    // Generation of the pixel buffer and tiles in flight. Used to verify inputs in update_tile.
    generation: u64,
    // Cached tiles for the current pixel buffer, in correct order for rendering
    // Also store the dimension of the cached tiles
    cached_tiles: Option<(u16, VecDeque<FilmTile>)>,
}

impl Film {
    /// Creates an empty `Film`.
    pub fn default() -> Self {
        Self {
            res: Vec2::new(4, 4),
            pixels: vec![Vec3::zeros(); 4 * 4],
            dirty: true,
            generation: 0,
            cached_tiles: None,
        }
    }

    /// Returns the resolution of the currently stored pixels of this `Film`.
    pub fn res(&self) -> Vec2<u16> {
        self.res
    }

    /// Returns the generation of the current pixel buffer and corresponding tiles.
    pub fn generation(&self) -> u64 {
        self.generation
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
    /// call to its [Film::clear_dirty].
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Returns blank `FilmTile`s for the buffer pixel with if they have been cached in the correct dimension.
    /// The returned tiles will be in the current generation.
    fn cached_tiles(&self, dim: u16) -> Option<VecDeque<FilmTile>> {
        if let Some((cached_dim, tiles)) = &self.cached_tiles {
            if *cached_dim == dim {
                let mut tiles = tiles.clone();
                for tile in &mut tiles {
                    tile.generation = self.generation;
                }
                Some(tiles)
            } else {
                None
            }
        } else {
            None
        }
    }

    fn cache_tiles(&mut self, tiles: &VecDeque<FilmTile>) {
        assert!(tiles.len() > 0);
        // Tile size is always at most the full resolution
        let dim = tiles[0].bb.diagonal().x;
        self.cached_tiles = Some((dim, tiles.clone()));
    }

    /// Resizes this `Film` according to `settings`.
    /// Note that this invalidates any tiles still held to the `Film`.
    fn resize(&mut self, settings: &FilmSettings) {
        assert!(settings.res.x >= settings.tile_dim && settings.res.y >= settings.tile_dim);

        // Bump generation for tile verification.
        self.generation += 1;

        self.res = settings.res;
        let pixel_count = (settings.res.x as usize) * (settings.res.y as usize);

        if self.pixels.len() != pixel_count || settings.clear {
            self.pixels = vec![settings.clear_color; pixel_count];
            self.cached_tiles = None;
            self.dirty = true;
        }
    }

    /// Updates this `Film` with the pixel values in a [FilmTile].
    pub fn update_tile(&mut self, tile: FilmTile) {
        if tile.generation != self.generation {
            yuki_warn!(
                "update_tile: Tile generation {} doesn't match film generation {}",
                tile.generation,
                self.generation
            );
            return;
        }

        let tile_min = tile.bb.p_min;
        let tile_max = tile.bb.p_max;

        if tile_max.x > self.res.x || tile_max.y > self.res.y {
            yuki_error!(
                "update_tile: Tile doesn't fit film ({:?} {:?})",
                self.res,
                tile.bb
            );
            return;
        }

        let tile_width = tile_max.x - tile_min.x;

        // Copy pixels over to the film
        // TODO: Accumulation, store counts per pixel
        for (tile_row, film_row) in ((tile_min.y as usize)..(tile_max.y as usize)).enumerate() {
            let film_row_offset = film_row * (self.res.x as usize);

            let film_slice_start = film_row_offset + (tile_min.x as usize);
            let film_slice_end = film_row_offset + (tile_max.x as usize);

            let tile_slice_start = tile_row * (tile_width as usize);
            let tile_slice_end = (tile_row + 1) * (tile_width as usize);

            let film_slice = &mut self.pixels[film_slice_start..film_slice_end];
            let tile_slice = &tile.pixels[tile_slice_start..tile_slice_end];

            film_slice.copy_from_slice(tile_slice);
        }
        self.dirty = true;
    }
}

fn generate_tiles(res: Vec2<u16>, tile_dim: u16, film_gen: u64) -> HashMap<(u16, u16), FilmTile> {
    // Collect tiles spanning the whole image hashed by their tile coordinates
    let mut tiles = HashMap::new();
    let dim = tile_dim;
    yuki_trace!("generate_tiles: Generating tiles");
    for j in (0..res.y).step_by(dim as usize) {
        for i in (0..res.x).step_by(dim as usize) {
            // Limit tiles to film dimensions
            let max_x = (i + dim).min(res.x);
            let max_y = (j + dim).min(res.y);

            tiles.insert(
                (i / dim, j / dim),
                FilmTile::new(
                    Bounds2::new(Point2::new(i, j), Point2::new(max_x, max_y)),
                    film_gen,
                ),
            );
        }
    }
    yuki_trace!("generate_tiles: Tiles generated");

    tiles
}

fn outward_spiral(
    mut tiles: HashMap<(u16, u16), FilmTile>,
    res: Vec2<u16>,
    tile_dim: u16,
) -> VecDeque<FilmTile> {
    // Algo adapted from https://stackoverflow.com/a/398302

    let tiles_x = ((res.x as f32) / (tile_dim as f32)).ceil() as i32;
    let tiles_y = ((res.y as f32) / (tile_dim as f32)).ceil() as i32;
    let center_x = (tiles_x / 2) - (1 - tiles_x % 2);
    let center_y = (tiles_y / 2) - (1 - tiles_y % 2);
    let max_dim = tiles_x.max(tiles_y);

    let mut x = 0;
    let mut y = 0;
    let mut dx = 0;
    let mut dy = -1;
    let mut tile_queue = VecDeque::new();
    yuki_trace!("outward_spiral: Collecting queue");
    for _ in 0..(max_dim * max_dim) {
        let tile_x = center_x + x;
        let tile_y = center_y + y;

        if tile_x >= 0 && tile_x < tiles_x && tile_y >= 0 && tile_y < tiles_y {
            tile_queue.push_back(tiles.remove(&(tile_x as u16, tile_y as u16)).unwrap());
        }

        if x == y || (x < 0 && x == -y) || (x > 0 && x == 1 - y) {
            std::mem::swap(&mut dx, &mut dy);
            dx *= -1;
        }

        x += dx;
        y += dy;
    }
    yuki_trace!("outward_spiral: Queue collected");

    if !tiles.is_empty() {
        yuki_warn!("outward_spiral: Dangling tiles: {:?}", tiles.keys());
    }

    tile_queue
}

/// Resizes the `Film` according to current `settings` if necessary and returns [FilmTile]s for rendering.
/// [FilmTile]s from previous calls should no longer be used.
pub fn film_tiles(film: &mut Arc<Mutex<Film>>, settings: &FilmSettings) -> VecDeque<FilmTile> {
    yuki_debug!("film_tiles: Begin");
    // Only lock the film for the duration of resizing
    let film_gen = {
        yuki_trace!("film_tiles: Waiting for lock on film");
        let mut film = film.lock().unwrap();
        yuki_trace!("film_tiles: Acquired film");

        yuki_trace!("film_tiles: Resizing film");
        film.resize(settings);
        let gen = film.generation();

        yuki_trace!("film_tiles: Releasing film");
        gen
    };

    let tiles = {
        yuki_trace!("film_tiles: Waiting for lock on film");
        let film = film.lock().unwrap();
        yuki_trace!("film_tiles: Acquired film");

        yuki_trace!("film_tiles: Checking for cached tiles");
        let tiles = film.cached_tiles(settings.tile_dim);

        yuki_trace!("film_tiles: Releasing film");
        tiles
    };
    let ret = if let Some(tiles) = tiles {
        yuki_debug!("film_tiles: Using cached tiles");
        tiles
    } else {
        yuki_trace!("film_tiles: Generating new tiles");
        let tiles = generate_tiles(settings.res, settings.tile_dim, film_gen);

        yuki_trace!("film_tiles: Ordering tiles");
        // Order tiles in a spiral from middle since that makes the visualisation more snappy:
        // Most things of interest are likely towards the center of the frame
        let tile_queue = outward_spiral(tiles, settings.res, settings.tile_dim);

        {
            yuki_trace!("film_tiles: Waiting for lock on film");
            let mut film = film.lock().unwrap();
            yuki_trace!("film_tiles: Acquired film");

            yuki_trace!("film_tiles: Caching tiles");
            film.cache_tiles(&tile_queue);

            yuki_trace!("film_tiles: Releasing film");
        }

        tile_queue
    };

    yuki_debug!("film_tiles: End");
    ret
}
