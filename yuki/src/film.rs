use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    math::{
        bounds::Bounds2,
        point::point2,
        vector::{Vec2, Vec3},
    },
    yuki_debug, yuki_error,
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
    cached_tiles: Option<(u16, Vec<FilmTile>)>,
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
    /// call to its [clear_dirty].
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Returns blank `FilmTile`s for the buffer pixel with if they have been cached in the correct dimension.
    /// The returned tiles will be in the current generation.
    fn cached_tiles(&self, dim: u16) -> Option<Vec<FilmTile>> {
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

    fn cache_tiles(&mut self, tiles: &Vec<FilmTile>) {
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
            yuki_error!(
                "Tile generation {} doesn't match film generation {}",
                tile.generation,
                self.generation
            );
            return;
        }

        let tile_min = tile.bb.p_min;
        let tile_max = tile.bb.p_max;

        if tile_max.x > self.res.x || tile_max.y > self.res.y {
            yuki_error!("Tile doesn't fit film ({:?} {:?})", self.res, tile.bb);
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

/// Resizes the `Film` according to current `settings` if necessary and returns [FilmTile]s for rendering.
/// [FilmTile]s from previous calls should no longer be used.
pub fn film_tiles(film: &mut Arc<Mutex<Film>>, settings: &FilmSettings) -> Vec<FilmTile> {
    // Only lock the film for the duration of resizing
    let film_gen = {
        yuki_debug!("Resizing film");
        let mut film = film.lock().unwrap();
        film.resize(settings);
        film.generation()
    };

    let tiles = {
        yuki_debug!("Checking for cached tiles");
        let film = film.lock().unwrap();
        film.cached_tiles(settings.tile_dim)
    };
    if let Some(tiles) = tiles {
        tiles
    } else {
        yuki_debug!("Generating tiles");
        // Collect tiles spanning the whole image hashed by their tile coordinates
        let mut tiles = HashMap::new();
        let dim = settings.tile_dim;
        for j in (0..settings.res.y).step_by(dim as usize) {
            for i in (0..settings.res.x).step_by(dim as usize) {
                // Limit tiles to film dimensions
                let max_x = (i + dim).min(settings.res.x);
                let max_y = (j + dim).min(settings.res.y);

                tiles.insert(
                    (i / dim, j / dim),
                    FilmTile::new(Bounds2::new(point2(i, j), point2(max_x, max_y)), film_gen),
                );
            }
        }

        yuki_debug!("Ordering tiles");
        // Order tiles in a spiral from middle since that makes the visualisation more snappy:
        // Most things of interest are likely towards the center of the frame
        let mut tile_queue = Vec::new();
        let tiles_x = ((settings.res.x as f32) / (dim as f32)).ceil() as i32;
        let tiles_y = ((settings.res.y as f32) / (dim as f32)).ceil() as i32;
        let max_dim = tiles_x.max(tiles_y);
        let center_x = tiles_x / 2;
        let center_y = tiles_y / 2;
        let mut x = 0;
        let mut y = 0;
        let mut dx = 0;
        let mut dy = -1;
        while !tiles.is_empty() {
            let tile_x = center_x + x;
            let tile_y = center_y + y;

            if (tile_x.abs() > max_dim) || (tile_y.abs() > max_dim) {
                yuki_error!(
                    "Tile spiral overflow at tile {}, {}!\nDangling tiles: {:?}",
                    tile_x,
                    tile_y,
                    tiles.keys()
                );
                break;
            }

            if tile_x >= 0 && tile_x < tiles_x && tile_y >= 0 && tile_y < tiles_y {
                tile_queue.push(tiles.remove(&(tile_x as u16, tile_y as u16)).unwrap());
            }

            if x == y || (x < 0 && x == -y) || (x > 0 && x == 1 - y) {
                std::mem::swap(&mut dx, &mut dy);
                dx *= -1;
            }

            x += dx;
            y += dy;
        }
        // Center tiles were pushed first
        tile_queue.reverse();

        {
            yuki_debug!("Caching tiles");
            let mut film = film.lock().unwrap();
            film.cache_tiles(&tile_queue);
        }

        tile_queue
    }
}
