mod bvh_heatmap;
mod normals;
mod whitted;

use bvh_heatmap::BVHIntersections;
use normals::Normals;
use whitted::Whitted;

use strum::{EnumString, EnumVariantNames, ToString};

use crate::{
    camera::{Camera, CameraSample},
    film::FilmTile,
    math::{Point2, Ray, Vec2, Vec3},
    sampling::Sampler,
    scene::Scene,
};

use std::sync::Arc;

pub type WhittedParams = whitted::Params;

#[derive(Copy, Clone, EnumVariantNames, ToString, EnumString)]
pub enum IntegratorType {
    Whitted(whitted::Params),
    BVHIntersections,
    Normals,
}

impl IntegratorType {
    pub fn instantiate(self) -> Box<dyn Integrator> {
        match self {
            IntegratorType::Whitted(params) => Box::new(Whitted::new(params)),
            IntegratorType::BVHIntersections => Box::new(BVHIntersections {}),
            IntegratorType::Normals => Box::new(Normals {}),
        }
    }
}

impl Default for IntegratorType {
    fn default() -> Self {
        IntegratorType::Whitted(whitted::Params::default())
    }
}

pub struct RadianceResult {
    pub li: Vec3<f32>,
    pub ray_scene_intersections: usize,
    pub rays: Vec<IntegratorRay>,
}

impl Default for RadianceResult {
    fn default() -> Self {
        Self {
            li: Vec3::from(0.0),
            ray_scene_intersections: 0,
            rays: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct IntegratorRay {
    pub ray: Ray<f32>,
    pub ray_type: RayType,
}

#[derive(Debug)]
pub enum RayType {
    Direct,
    Reflection,
    Refraction,
    Normal,
    Shadow,
}

// Public interface for scene integrators.
pub trait Integrator {
    /// Evaluates the incoming radiance along `ray`. Also returns the number of rays intersected with `scene`.
    /// If called with `collect_rays` true, populates the list of rays launched.
    fn li(&self, ray: Ray<f32>, scene: &Scene, depth: u32, collect_rays: bool) -> RadianceResult;

    /// Renders the given `Tile`. Returns the number of rays intersected with `scene`.
    fn render(
        &self,
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

                let ray = camera.ray(&CameraSample { p_film });

                let result = self.li(ray, scene, 0, false);
                color += result.li;
                ray_count += result.ray_scene_intersections;
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
}
