use super::{Integrator, IntegratorRay, RadianceResult, RayType};
use crate::{
    bvh::IntersectionResult,
    interaction::{Interaction, SurfaceInteraction},
    lights::LightSample,
    materials::{Bsdf, BxdfSample, BxdfType},
    math::{Point2, Ray, Spectrum},
    sampling::Sampler,
    scene::Scene,
    shapes::Hit,
};

use allocators::ScopedScratch;
use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct Params {
    pub max_depth: u32,
}

impl Default for Params {
    fn default() -> Self {
        Self { max_depth: 3 }
    }
}

pub struct Whitted {
    max_depth: u32,
}

impl Whitted {
    pub fn new(params: Params) -> Self {
        Self {
            max_depth: params.max_depth,
        }
    }

    fn specular_contribution(
        &self,
        scratch: &ScopedScratch,
        si: &SurfaceInteraction,
        bsdf: &Bsdf,
        scene: &Scene,
        depth: u32,
        sampler: &mut Box<dyn Sampler>,
        ray_type: BxdfType,
        rays: Option<&mut Vec<IntegratorRay>>,
    ) -> RadianceResult {
        let BxdfSample {
            wi, f, sample_type, ..
        } = bsdf.sample_f(si.wo, Point2::new(0.0, 0.0), BxdfType::SPECULAR | ray_type);
        if sample_type == BxdfType::NONE {
            RadianceResult::default()
        } else {
            let refl = Interaction::from(si).spawn_ray(wi);

            let mut ret = self.li_internal(
                scratch,
                refl,
                scene,
                depth + 1,
                sampler,
                rays,
                sample_type.contains(BxdfType::SPECULAR),
            );
            ret.li = f * ret.li * wi.dot_n(si.shading.n).abs();

            ret
        }
    }

    // Always inline to have the compiler strip out ray collection in li()-calls
    #[inline(always)]
    fn li_internal(
        &self,
        scratch: &ScopedScratch,
        ray: Ray<f32>,
        scene: &Scene,
        depth: u32,
        sampler: &mut Box<dyn Sampler>,
        mut rays: Option<&mut Vec<IntegratorRay>>,
        is_specular: bool,
    ) -> RadianceResult {
        let IntersectionResult { hit, .. } = scene.bvh.intersect(ray);

        let min_debug_ray_length = {
            let bounds = scene.bvh.bounds();
            let i = bounds.maximum_extent();
            (bounds.p_max[i] - bounds.p_min[i]) / 10.0
        };
        if let Some(collected_rays) = &mut rays {
            collected_rays.push(IntegratorRay {
                ray: Ray::new(ray.o, ray.d, ray.t_max),
                ray_type: RayType::Direct,
            });
        }
        let (incoming_radiance, ray_count) = if let Some(Hit { si, t, shape, .. }) = hit {
            if let Some(collected_rays) = &mut rays {
                collected_rays.last_mut().unwrap().ray.t_max = t;
                collected_rays.push(IntegratorRay {
                    ray: Ray::new(si.p, si.n.into(), min_debug_ray_length),
                    ray_type: RayType::Normal,
                });
            }

            let bsdf = shape.compute_scattering_functions(scratch, &si);

            let mut ray_count = 1;
            let mut sum_li = scene.lights.iter().fold(Spectrum::zeros(), |c, l| {
                let LightSample { l, li, vis, pdf } = l.sample_li(&si, sampler.get_2d());
                if !li.is_black() {
                    let f = bsdf.f(si.wo, l, BxdfType::all());
                    if let Some(test) = vis {
                        if let Some(collected_rays) = &mut rays {
                            collected_rays.push(IntegratorRay {
                                ray: test.ray(),
                                ray_type: RayType::Shadow,
                            });
                        }
                        if !f.is_black() && test.unoccluded(scene) {
                            return c + f * li * si.shading.n.dot_v(l).clamp(0.0, 1.0) / pdf;
                        }
                    }
                }
                c
            });

            if depth == 0 || is_specular {
                sum_li += si.emitted_radiance(-ray.d);
            }

            if depth + 1 < self.max_depth {
                macro_rules! spec {
                    ($t:expr, $rt:expr) => {{
                        if let Some(collected_rays) = &mut rays {
                            let mut child_rays = Vec::new();
                            let RadianceResult {
                                li,
                                ray_scene_intersections,
                            } = self.specular_contribution(
                                scratch,
                                &si,
                                &bsdf,
                                scene,
                                depth,
                                sampler,
                                $t,
                                Some(&mut child_rays),
                            );
                            sum_li += li;
                            ray_count += ray_scene_intersections;
                            if !child_rays.is_empty() {
                                child_rays[0].ray_type = $rt;
                                collected_rays.append(&mut child_rays);
                            }
                        } else {
                            let RadianceResult {
                                li,
                                ray_scene_intersections,
                            } = self.specular_contribution(
                                scratch, &si, &bsdf, scene, depth, sampler, $t, None,
                            );
                            sum_li += li;
                            ray_count += ray_scene_intersections;
                        }
                    }};
                }
                spec!(BxdfType::REFLECTION, RayType::Reflection);
                spec!(BxdfType::TRANSMISSION, RayType::Refraction);
            }

            (sum_li, ray_count)
        } else {
            (scene.background, 1)
        };

        RadianceResult {
            li: incoming_radiance,
            ray_scene_intersections: ray_count,
        }
    }
}

impl Integrator for Whitted {
    fn li(
        &self,
        scratch: &ScopedScratch,
        ray: Ray<f32>,
        scene: &Scene,
        depth: u32,
        sampler: &mut Box<dyn Sampler>,
    ) -> RadianceResult {
        self.li_internal(scratch, ray, scene, depth, sampler, None, false)
    }

    fn li_debug(
        &self,
        scratch: &ScopedScratch,
        ray: Ray<f32>,
        scene: &Scene,
        depth: u32,
        sampler: &mut Box<dyn Sampler>,
        rays: &mut Vec<IntegratorRay>,
    ) -> RadianceResult {
        self.li_internal(scratch, ray, scene, depth, sampler, Some(rays), false)
    }
}
