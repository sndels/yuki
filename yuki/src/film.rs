use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

use crate::{
    math::{Bounds2, Point2, Spectrum, Vec2},
    yuki_debug, yuki_error, yuki_trace, yuki_warn,
};

/// The settings for a `Film`.
#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub struct FilmSettings {
    /// The total film resolution.
    pub res: Vec2<u16>,
    /// The tile size to be used.
    pub tile_dim: u16,
    /// `true` if pixels need to be cleared even if the buffer is not resized
    pub clear: bool,
    /// `true` if film is used to accumulate over time instead of writing final
    /// pixel value immediately.
    pub accumulate: bool,
    /// `true` if render should run in 1/16th res (both dimensions divided by 4)
    pub sixteenth_res: bool,
}

impl Default for FilmSettings {
    /// Creates a new `FilmSettings` with `res` [640, 480], `tile_dim` [8,8] and clearing on with black color.
    fn default() -> Self {
        Self {
            res: Vec2::new(640, 480),
            tile_dim: 16,
            clear: true,
            accumulate: false,
            sixteenth_res: false,
        }
    }
}

/// A film tile used for rendering.
#[derive(Debug, Clone)]
pub struct FilmTile {
    /// The [Film] pixel bounds for this tile.
    pub bb: Bounds2<u16>,
    pub sample: u16,
    // Flat index in samples list
    index: usize,
    // Generation of this tile.
    generation: u64,
    film_id: u32,
}

impl FilmTile {
    /// Creates a new `FilmTile` with the given [Bounds2].
    pub fn new(bb: Bounds2<u16>, index: usize, sample: u16, generation: u64, film_id: u32) -> Self {
        FilmTile {
            bb,
            index,
            sample,
            generation,
            film_id,
        }
    }
}

/// Pixel wrapper for rendering through [FilmTile]s.
pub struct Film {
    // Resolution of the stored pixel buffer.
    res: Vec2<u16>,
    // Pixel values.
    pixels: Vec<Spectrum<f32>>,
    // Sample count for each tile.
    samples: Option<Vec<u32>>,
    // Indicator for changed pixel values.
    dirty: bool,
    // Generation of the pixel buffer and tiles in flight.
    generation: u64,
    // Random identifier for the film itself.
    id: u32,
    // Cached tiles for the current pixel buffer, in correct order for rendering
    tile_cache: Option<TileCache>,
}

struct TileCache {
    dim: u16,
    tiles: VecDeque<FilmTile>,
}

impl Film {
    /// Creates a new `Film`.
    pub fn new(res: Vec2<u16>) -> Self {
        Self {
            res,
            pixels: vec![Spectrum::zeros(); (res.x as usize) * (res.y as usize)],
            samples: None,
            dirty: true,
            generation: 0,
            id: rand::random::<u32>(),
            tile_cache: None,
        }
    }

    /// Returns the resolution of the currently stored pixels of this `Film`.
    pub fn res(&self) -> Vec2<u16> {
        self.res
    }

    /// Returns the generation of the current pixel buffer and corresponding tiles.
    fn move_generation(&mut self) {
        self.generation += 1;
    }

    /// Returns a reference to the the pixels of this `Film`.
    pub fn pixels(&self) -> &Vec<Spectrum<f32>> {
        &self.pixels
    }

    /// Returns a reference to the the samples for each tile in this `Film`.
    pub fn samples(&self) -> Option<&Vec<u32>> {
        self.samples.as_ref()
    }

    /// Clears the indicator for changed pixel values in this `Film`.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Returns `true` if this `Film`s pixels have been written to since the last
    /// call to its [`Film::clear_dirty`].
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Returns `true` if `tile` belongs to this `Film` and is from the same generation.
    pub fn matches(&self, tile: &FilmTile) -> bool {
        assert!(self.id == tile.film_id, "Tile-Film ID mismatch");

        self.id == tile.film_id && self.generation == tile.generation
    }

    /// Returns the dimensions of the active tiles.
    pub fn tile_dim(&self) -> Option<u16> {
        if let Some(TileCache { dim, .. }) = self.tile_cache.as_ref() {
            Some(*dim)
        } else {
            None
        }
    }

    /// Returns blank `FilmTile`s for the buffer pixel with if they have been cached in the correct dimension.
    /// The returned tiles will be in the current generation.
    fn cached_tiles(&self, dim: u16) -> Option<VecDeque<FilmTile>> {
        if let Some(TileCache {
            dim: cached_dim,
            tiles,
        }) = &self.tile_cache
        {
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
        assert!(!tiles.is_empty());
        // Tile size is always at most the full resolution
        let dim = tiles[0].bb.diagonal().x;
        self.tile_cache = Some(TileCache {
            dim,
            tiles: tiles.clone(),
        });
    }

    /// Circles the given tile in this `Film` with a pixel border of `color`.
    pub fn mark(&mut self, tile: &FilmTile, color: Spectrum<f32>) {
        let (left, top, right, bottom) = (
            tile.bb.p_min.x as usize,
            tile.bb.p_min.y as usize,
            tile.bb.p_max.x as usize,
            tile.bb.p_max.y as usize,
        );

        for j in &[top, bottom - 1] {
            let row_first = j * (self.res.x as usize);
            for i in left..right {
                self.pixels[row_first + i] = color;
            }
        }

        for i in &[left, right - 1] {
            for j in top..bottom {
                let row_first = j * (self.res.x as usize);
                self.pixels[row_first + i] = color;
            }
        }

        self.dirty = true;
    }

    /// Updates this `Film` with the pixel values in a [`FilmTile`].
    pub fn update_tile(&mut self, tile: &FilmTile, tile_pixels: &[Spectrum<f32>]) {
        assert!(tile_pixels.len() >= tile.bb.area() as usize);

        if !self.matches(tile) {
            yuki_warn!(
                "update_tile: Tile {}:{} doesn't match film {}:{}",
                tile.film_id,
                tile.generation,
                self.id,
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

        macro_rules! update_slices {
            ($write_expr:expr) => {
                // Copy pixels over to the film
                for (tile_row, film_row) in
                    ((tile_min.y as usize)..(tile_max.y as usize)).enumerate()
                {
                    let film_row_offset = film_row * (self.res.x as usize);

                    let film_slice_start = film_row_offset + (tile_min.x as usize);
                    let film_slice_end = film_row_offset + (tile_max.x as usize);

                    let tile_slice_start = tile_row * (tile_width as usize);
                    let tile_slice_end = (tile_row + 1) * (tile_width as usize);

                    let film_slice = &mut self.pixels[film_slice_start..film_slice_end];
                    let tile_slice = &tile_pixels[tile_slice_start..tile_slice_end];

                    $write_expr(film_slice, tile_slice);
                }
            };
        }

        if let Some(samples) = &mut self.samples {
            update_slices!(
                |film_slice: &mut [Spectrum<f32>], tile_slice: &[Spectrum<f32>]| {
                    film_slice
                        .iter_mut()
                        .zip(tile_slice.iter())
                        .for_each(|(fc, &c)| {
                            *fc += c;
                        });
                }
            );

            samples[tile.index] += 1;
        } else {
            update_slices!(
                |film_slice: &mut [Spectrum<f32>], tile_slice: &[Spectrum<f32>]| {
                    film_slice.copy_from_slice(tile_slice);
                }
            );
        }

        self.dirty = true;
    }
}

impl Default for Film {
    fn default() -> Self {
        Self {
            res: Vec2::new(4, 4),
            pixels: vec![Spectrum::zeros(); 4 * 4],
            samples: None,
            dirty: true,
            generation: 0,
            tile_cache: None,
            id: rand::random::<u32>(),
        }
    }
}

fn generate_tiles(
    res: Vec2<u16>,
    tile_dim: u16,
    film_gen: u64,
    film_id: u32,
) -> HashMap<(u16, u16), FilmTile> {
    // Collect tiles spanning the whole image hashed by their tile coordinates
    let mut tiles = HashMap::new();
    yuki_trace!("generate_tiles: Generating tiles");
    let mut flat_index = 0usize;
    for j in (0..res.y).step_by(tile_dim as usize) {
        for i in (0..res.x).step_by(tile_dim as usize) {
            // Limit tiles to film dimensions
            let max_x = (i + tile_dim).min(res.x);
            let max_y = (j + tile_dim).min(res.y);

            tiles.insert(
                (i / tile_dim, j / tile_dim),
                FilmTile::new(
                    Bounds2::new(Point2::new(i, j), Point2::new(max_x, max_y)),
                    flat_index,
                    0,
                    film_gen,
                    film_id,
                ),
            );
            flat_index += 1;
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

    let h_tiles = ((res.x as f32) / (tile_dim as f32)).ceil() as i32;
    let v_tiles = ((res.y as f32) / (tile_dim as f32)).ceil() as i32;
    let center_x = (h_tiles / 2) - (1 - h_tiles % 2);
    let center_y = (v_tiles / 2) - (1 - v_tiles % 2);
    let max_dim = h_tiles.max(v_tiles);

    let mut x = 0;
    let mut y = 0;
    let mut dx = 0;
    let mut dy = -1;
    let mut tile_queue = VecDeque::new();
    yuki_trace!("outward_spiral: Collecting queue");
    for _ in 0..(max_dim * max_dim) {
        let tile_x = center_x + x;
        let tile_y = center_y + y;

        if tile_x >= 0 && tile_x < h_tiles && tile_y >= 0 && tile_y < v_tiles {
            #[allow(clippy::cast_sign_loss)] // We check above
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

pub fn film_or_new(film: &Arc<Mutex<Film>>, settings: FilmSettings) -> Arc<Mutex<Film>> {
    yuki_trace!("film_or_new: Waiting for lock on film (res)");
    let film_res = film.lock().unwrap().res();
    yuki_trace!("film_or_new: Acquired and released film (res)");

    if settings.clear || film_res != settings.res {
        assert!(
            settings.res.x >= settings.tile_dim && settings.res.y >= settings.tile_dim,
            "Film resolution is smaller than tile size"
        );

        yuki_trace!("film_or_new: Creating new film");

        let new_film = Arc::new(Mutex::new(Film::new(settings.res)));
        yuki_trace!("film_or_new: Releasing film");
        new_film
    } else {
        yuki_trace!("film_or_new: Waiting for lock on film (move_generation)");
        film.lock().unwrap().move_generation();
        yuki_trace!("film_or_new: Acquired and released film (move_generation)");

        yuki_debug!(
            "film_or_new: New film generation {}",
            film.lock().unwrap().generation
        );

        Arc::clone(film)
    }
}

/// Generates [FilmTile]s for rendering.
pub fn film_tiles(film: &mut Arc<Mutex<Film>>, settings: FilmSettings) -> VecDeque<FilmTile> {
    yuki_debug!("film_tiles: Begin");

    let tiles = {
        yuki_trace!("film_tiles: Waiting for lock on film");
        let film = film.lock().unwrap();
        yuki_trace!("film_tiles: Acquired film");

        assert!(film.res() == settings.res, "Film does not match settings");

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
        let (generation, id) = {
            yuki_trace!("film_tiles: Waiting for lock on film");
            let film = film.lock().unwrap();
            yuki_trace!("film_tiles: Acquired film");

            yuki_trace!("film_tiles: Releasing film");
            (film.generation, film.id)
        };
        let tiles = generate_tiles(settings.res, settings.tile_dim, generation, id);

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

    {
        yuki_trace!("film_tiles: Waiting for lock on film");
        let mut film = film.lock().unwrap();
        yuki_trace!("film_tiles: Acquired film");

        if settings.accumulate {
            film.samples = Some(vec![0; ret.len()]);
        } else {
            film.samples = None;
        }

        yuki_trace!("film_tiles: Releasing film");
    }

    yuki_debug!("film_tiles: End");
    ret
}
