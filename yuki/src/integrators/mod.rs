mod bvh_heatmap;
mod geometry_normals;
mod path;
mod shading_normals;
mod whitted;

use bvh_heatmap::BVHIntersections;
use geometry_normals::GeometryNormals;
use path::Path;
use shading_normals::ShadingNormals;
use whitted::Whitted;

use allocators::ScopedScratch;
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString, EnumVariantNames};

use crate::{
    camera::{Camera, CameraSample},
    film::FilmTile,
    math::{Point2, Ray, Spectrum, Vec2},
    sampling::Sampler,
    scene::Scene,
};

use std::sync::Arc;

pub type WhittedParams = whitted::Params;
pub type PathParams = path::Params;

#[derive(Copy, Clone, Deserialize, Serialize, Display, EnumVariantNames, EnumString)]
pub enum IntegratorType {
    Whitted(whitted::Params),
    Path(path::Params),
    BVHIntersections,
    GeometryNormals,
    ShadingNormals,
}

impl IntegratorType {
    pub fn instantiate(self) -> Box<dyn Integrator> {
        match self {
            IntegratorType::Whitted(params) => Box::new(Whitted::new(params)),
            IntegratorType::Path(params) => Box::new(Path::new(params)),
            IntegratorType::BVHIntersections => Box::new(BVHIntersections {}),
            IntegratorType::GeometryNormals => Box::new(GeometryNormals {}),
            IntegratorType::ShadingNormals => Box::new(ShadingNormals {}),
        }
    }

    pub fn n_sampled_dimensions(self) -> usize {
        match self {
            IntegratorType::Path(PathParams { max_depth, .. }) => (max_depth.max(1) - 1) as usize, // Bounce dir, russian roulette per bounce
            IntegratorType::Whitted(_)
            | IntegratorType::BVHIntersections
            | IntegratorType::GeometryNormals
            | IntegratorType::ShadingNormals => 0,
        }
    }
}

#[allow(clippy::derivable_impls)] // Can't derive Default for non unit variants, which Whitted is
impl Default for IntegratorType {
    fn default() -> Self {
        IntegratorType::Whitted(whitted::Params::default())
    }
}

pub struct RadianceResult {
    pub li: Spectrum<f32>,
    pub ray_scene_intersections: usize,
    pub rays: Vec<IntegratorRay>,
}

impl Default for RadianceResult {
    fn default() -> Self {
        Self {
            li: Spectrum::zeros(),
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

#[derive(Copy, Clone, Debug, PartialEq)]
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
    fn li(
        &self,
        scratch: &ScopedScratch,
        ray: Ray<f32>,
        scene: &Scene,
        depth: u32,
        sampler: &mut Box<dyn Sampler>,
        collect_rays: bool,
    ) -> RadianceResult;

    /// Renders the given `Tile`. Returns the number of rays intersected with `scene`.
    fn render(
        &self,
        scratch: &ScopedScratch,
        scene: &Scene,
        camera: &Camera,
        sampler: &Arc<dyn Sampler>,
        accumulating: bool,
        tile: &mut FilmTile,
        tile_pixels: &mut [Spectrum<f32>],
        early_termination_predicate: &mut dyn FnMut() -> bool,
    ) -> usize {
        assert!(tile_pixels.len() >= tile.bb.area() as usize);

        let tile_width = tile.bb.width();

        // Init per tile to try and get as deterministic results as possible between runs
        // This makes the rng the same per tile regardless of which threads take which tiles
        // Of course, this is useful only for debug but the init hit is miniscule in comparison to render time
        let corner = tile.bb.p_min;
        #[allow(unused_comparisons)] // Just in case these are changed from u16 for some reason
        let types_fit = (tile.sample <= 0xFFFF) && (corner.x <= 0xFFFF) && (corner.y <= 0xFFFF);
        assert!(types_fit);
        let mut sampler = sampler
            .as_ref()
            .clone(((tile.sample as u64) << 32) | ((corner.x as u64) << 16) | (corner.y as u64));
        // TODO:
        // Better way to have samples distributed over multiple accumulation iterations?
        // start_pixel() is now >60% of the work done by a worker in an accumulating render

        let mut ray_count = 0;
        for p in tile.bb {
            sampler.start_pixel();
            let mut color = Spectrum::zeros();
            let sample_count = if accumulating {
                1
            } else {
                sampler.samples_per_pixel()
            };
            for _ in 0..sample_count {
                if early_termination_predicate() {
                    return ray_count;
                }

                if tile.sample > 0 {
                    // NOTE:
                    // The generated samples won't match between accumulating and non-accumulating
                    // rendering as start_pixel is called after different numbers of extra generated
                    // prgn values. This call only ensures that the samples are well distributed
                    // between accumulating iterations. 1:1 can be verified by setting a large enough
                    // sampled dimension count and removing tile sample from the cloning seed
                    sampler.set_sample(tile.sample as usize);
                } else {
                    sampler.start_sample();
                }

                let sample_scratch = ScopedScratch::new_scope(scratch);

                let p_film = Point2::new(p.x as f32, p.y as f32) + sampler.get_2d();

                let ray = camera.ray(&CameraSample { p_film });

                let result = self.li(&sample_scratch, ray, scene, 0, &mut sampler, false);
                color += result.li;
                ray_count += result.ray_scene_intersections;
            }
            color /= sample_count as f32;

            let Vec2 {
                x: tile_x,
                y: tile_y,
            } = p - tile.bb.p_min;
            let pixel_offset = (tile_y * tile_width + tile_x) as usize;
            tile_pixels[pixel_offset] = color;
        }
        ray_count
    }
}
