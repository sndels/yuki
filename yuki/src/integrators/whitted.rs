use super::{Integrator, IntegratorRay, RadianceResult, RayType};
use crate::{
    bvh::IntersectionResult,
    interaction::{Interaction, SurfaceInteraction},
    lights::LightSample,
    materials::{BxdfSample, BxdfType},
    math::{Point2, Ray, Spectrum},
    sampling::Sampler,
    scene::Scene,
    shapes::Hit,
};

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
        si: &SurfaceInteraction,
        scene: &Scene,
        depth: u32,
        sampler: &mut Box<dyn Sampler>,
        ray_type: BxdfType,
        collect_rays: bool,
    ) -> RadianceResult {
        let BxdfSample {
            wi, f, sample_type, ..
        } = si.bsdf.as_ref().unwrap().sample_f(
            si.wo,
            Point2::new(0.0, 0.0),
            BxdfType::SPECULAR | ray_type,
        );
        if sample_type == BxdfType::NONE {
            RadianceResult::default()
        } else {
            let refl = Interaction::from(si).spawn_ray(wi);

            let mut ret = self.li(refl, scene, depth + 1, sampler, collect_rays);
            ret.li = f * ret.li * wi.dot_n(si.shading.n).abs();

            ret
        }
    }
}

impl Integrator for Whitted {
    fn li(
        &self,
        ray: Ray<f32>,
        scene: &Scene,
        depth: u32,
        sampler: &mut Box<dyn Sampler>,
        collect_rays: bool,
    ) -> RadianceResult {
        let IntersectionResult { hit, .. } = scene.bvh.intersect(ray);

        let min_debug_ray_length = {
            let bounds = scene.bvh.bounds();
            let i = bounds.maximum_extent();
            (bounds.p_max[i] - bounds.p_min[i]) / 10.0
        };
        let mut collected_rays: Vec<IntegratorRay> = if collect_rays {
            vec![IntegratorRay {
                ray: Ray::new(
                    ray.o,
                    ray.d,
                    scene
                        .bvh
                        .bounds()
                        .intersections(ray)
                        .map_or(min_debug_ray_length, |(_, t_max)| t_max),
                ),
                ray_type: RayType::Direct,
            }]
        } else {
            Vec::new()
        };
        let (incoming_radiance, ray_count) = if let Some(Hit { si, t, .. }) = hit {
            if collect_rays {
                collected_rays[0].ray.t_max = t;
                collected_rays.push(IntegratorRay {
                    ray: Ray::new(si.p, si.n.into(), min_debug_ray_length),
                    ray_type: RayType::Normal,
                });
            }

            let mut ray_count = 1;
            let mut sum_li = scene.lights.iter().fold(Spectrum::zeros(), |c, l| {
                let LightSample { l, li, vis } = l.sample_li(&si);
                if !li.is_black() {
                    let f = si.bsdf.as_ref().unwrap().f(si.wo, l, BxdfType::all());
                    if let Some(test) = vis {
                        if collect_rays {
                            collected_rays.push(IntegratorRay {
                                ray: test.ray(),
                                ray_type: RayType::Shadow,
                            });
                        }
                        if !f.is_black() && test.unoccluded(scene) {
                            return c + f * li * si.shading.n.dot_v(l).clamp(0.0, 1.0);
                        }
                    }
                }
                c
            });

            if depth + 1 < self.max_depth {
                macro_rules! spec {
                    ($t:expr, $rt:expr) => {{
                        let RadianceResult {
                            li,
                            ray_scene_intersections,
                            mut rays,
                        } = self.specular_contribution(
                            &si,
                            scene,
                            depth,
                            sampler,
                            $t,
                            collect_rays,
                        );
                        sum_li += li;
                        ray_count += ray_scene_intersections;
                        if !rays.is_empty() {
                            rays[0].ray_type = $rt;
                            collected_rays.append(&mut rays);
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
            rays: collected_rays,
        }
    }
}
