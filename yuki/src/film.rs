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
}

impl FilmSettings {
    /// Creates a new `FilmSettings` with `res` [640, 480], `tile_dim` [8,8] and clearing on with black color.
    pub fn default() -> Self {
        Self {
            res: Vec2::new(640, 480),
            tile_dim: 16,
            clear: true,
        }
    }
}

/// A film tile used for rendering.
#[derive(Debug, Clone)]
pub struct FilmTile {
    /// The [Film] pixel bounds for this tile.
    pub bb: Bounds2<u16>,
    /// Pixel values in this tile stored in row-major RGB order.
    pub pixels: Vec<Spectrum<f32>>,
    // Generation of this tile.
    generation: u64,
    film_id: u32,
}

impl FilmTile {
    /// Creates a new `FilmTile` with the given [Bounds2].
    pub fn new(bb: Bounds2<u16>, generation: u64, film_id: u32) -> Self {
        let width = (bb.p_max.x - bb.p_min.x) as usize;
        let height = (bb.p_max.y - bb.p_min.y) as usize;

        FilmTile {
            bb,
            pixels: vec![Spectrum::zeros(); width * height],
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
    // Indicator for changed pixel values.
    dirty: bool,
    // Generation of the pixel buffer and tiles in flight.
    generation: u64,
    // Random identifier for the film itself.
    id: u32,
    // Cached tiles for the current pixel buffer, in correct order for rendering
    // Also store the dimension of the cached tiles
    cached_tiles: Option<(u16, VecDeque<FilmTile>)>,
}

impl Film {
    /// Creates a new `Film`.
    pub fn new(res: Vec2<u16>) -> Self {
        Self {
            res,
            pixels: vec![Spectrum::zeros(); (res.x as usize) * (res.y as usize)],
            dirty: true,
            generation: 0,
            id: rand::random::<u32>(),
            cached_tiles: None,
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
        if self.id != tile.film_id {
            // Since tiles and their film go hand in hand, this should never happen
            panic!("Tile-Film ID mismatch");
        }
        self.id == tile.film_id && self.generation == tile.generation
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
        assert!(!tiles.is_empty());
        // Tile size is always at most the full resolution
        let dim = tiles[0].bb.diagonal().x;
        self.cached_tiles = Some((dim, tiles.clone()));
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
    pub fn update_tile(&mut self, tile: &FilmTile) {
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

impl Default for Film {
    fn default() -> Self {
        Self {
            res: Vec2::new(4, 4),
            pixels: vec![Spectrum::zeros(); 4 * 4],
            dirty: true,
            generation: 0,
            cached_tiles: None,
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
                    film_id,
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

    yuki_debug!("film_tiles: End");
    ret
}
