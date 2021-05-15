mod bvh_heatmap;
mod whitted;

pub use bvh_heatmap::BVHIntersectionsIntegrator;
pub use whitted::WhittedIntegrator;

use crate::{
    camera::{Camera, CameraSample},
    film::FilmTile,
    math::{Point2, Ray, Vec2, Vec3},
    samplers::Sampler,
    scene::Scene,
};

use std::sync::Arc;

pub trait Integrator {
    /// Renders the given `Tile`. Returns the number of rays intersected with `scene`.
    fn render(
        scene: &Scene,
        camera: &Camera,
        sampler: &Arc<dyn Sampler>,
        tile: &mut FilmTile,
        early_termination_predicate: &mut dyn FnMut() -> bool,
    ) -> usize {
        let tile_width = tile.bb.p_max.x - tile.bb.p_min.x;
        // Init per tile to try and get as deterministic results as possible between runs
        // This makes the rng the same per tile regardless of which threads take which tiles
        // Of course, this is useful only for debug but the init hit is miniscule in comparison to render time
        let mut sampler = sampler
            .as_ref()
            .clone(((tile.bb.p_min.x as u64) << 32) & (tile.bb.p_min.y as u64));

        let mut ray_count = 0;
        for p in tile.bb {
            sampler.start_pixel();
            let mut color = Vec3::from(0.0);
            for _ in 0..sampler.samples_per_pixel() {
                if early_termination_predicate() {
                    return ray_count;
                }

                sampler.start_sample();

                let p_film = Point2::new(p.x as f32, p.y as f32) + sampler.get_2d();

                let ray = camera.ray(CameraSample { p_film });

                let (li, rc) = Self::li(ray, scene);
                color += li;
                ray_count += rc;
            }
            color /= sampler.samples_per_pixel() as f32;

            let Vec2 {
                x: tile_x,
                y: tile_y,
            } = p - tile.bb.p_min;
            let pixel_offset = (tile_y * tile_width + tile_x) as usize;
            tile.pixels[pixel_offset] = color;
        }
        ray_count
    }

    /// Evaluates the incoming radiance along `ray`. Also returns the number of rays intersected with `scene`.
    fn li(ray: Ray<f32>, scene: &Scene) -> (Vec3<f32>, usize);
}