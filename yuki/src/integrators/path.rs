use super::{Integrator, IntegratorRay, RadianceResult, RayType};
use crate::{
    bvh::IntersectionResult,
    interaction::Interaction,
    lights::LightSample,
    materials::{BxdfSample, BxdfType},
    math::{Ray, Spectrum},
    sampling::Sampler,
    scene::Scene,
    shapes::Hit,
};

use allocators::ScopedScratch;
use serde::{Deserialize, Serialize};

// Based on Physically Based Rendering 3rd ed.
// https://www.pbr-book.org/3ed-2018/Light_Transport_I_Surface_Reflection/Path_Tracing

#[derive(Copy, Clone, Deserialize, Serialize)]
pub struct Params {
    pub max_depth: u32,
    pub indirect_clamp: Option<f32>,
}

impl Default for Params {
    fn default() -> Self {
        Self {
            max_depth: 3,
            indirect_clamp: None,
        }
    }
}

pub struct Path {
    max_depth: u32,
    indirect_clamp: Option<f32>,
}

impl Path {
    pub fn new(params: Params) -> Self {
        Self {
            max_depth: params.max_depth,
            indirect_clamp: params.indirect_clamp,
        }
    }
}

impl Integrator for Path {
    fn li(
        &self,
        scratch: &ScopedScratch,
        mut ray: Ray<f32>,
        scene: &Scene,
        _depth: u32,
        sampler: &mut Box<dyn Sampler>,
        collect_rays: bool,
    ) -> RadianceResult {
        let min_debug_ray_length = {
            let bounds = scene.bvh.bounds();
            let i = bounds.maximum_extent();
            (bounds.p_max[i] - bounds.p_min[i]) / 10.0
        };

        let mut collected_rays = Vec::new();
        let mut incoming_radiance = Spectrum::zeros();
        let mut beta = Spectrum::ones();
        let mut bounces = 0;
        let mut specular_bounce = false;
        let mut ray_count = 0;
        let mut ray_type = RayType::Direct;
        while bounces < self.max_depth {
            if collect_rays {
                collected_rays.push(IntegratorRay {
                    ray: Ray::new(
                        ray.o,
                        ray.d,
                        scene
                            .bvh
                            .bounds()
                            .intersections(ray)
                            .map_or(min_debug_ray_length, |(_, t_max)| t_max),
                    ),
                    ray_type,
                });
            }
            ray_count += 1;

            let IntersectionResult { hit, .. } = scene.bvh.intersect(scratch, ray);

            if let Some(Hit { si, t, bsdf }) = hit {
                if collect_rays {
                    collected_rays[0].ray.t_max = t;
                    collected_rays.push(IntegratorRay {
                        ray: Ray::new(si.p, si.n.into(), min_debug_ray_length),
                        ray_type: RayType::Normal,
                    });
                }

                let mut radiance = scene.lights.iter().fold(Spectrum::zeros(), |c, l| {
                    let LightSample { l, li, vis, pdf } = l.sample_li(&si, sampler.get_2d());
                    if !li.is_black() {
                        let f = bsdf.as_ref().unwrap().f(si.wo, l, BxdfType::all());
                        if let Some(test) = vis {
                            if collect_rays {
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

                if bounces == 0 || specular_bounce {
                    radiance += beta * si.emitted_radiance(-ray.d);
                }

                if bounces > 0 && self.indirect_clamp.is_some() {
                    radiance = radiance.min(Spectrum::ones() * self.indirect_clamp.unwrap());
                }

                incoming_radiance += beta * radiance;

                let wo = -ray.d;
                let BxdfSample {
                    wi,
                    f,
                    pdf,
                    sample_type,
                } = bsdf
                    .as_ref()
                    .unwrap()
                    .sample_f(wo, sampler.get_2d(), BxdfType::all());

                if f.is_black() || pdf == 0.0 {
                    break;
                }
                specular_bounce = sample_type.contains(BxdfType::SPECULAR);

                beta *= f * wi.dot_n(si.shading.n).abs() / pdf;
                ray = Interaction::from(&si).spawn_ray(wi);
                ray_type = if sample_type.contains(BxdfType::REFLECTION) {
                    RayType::Reflection
                } else if sample_type.contains(BxdfType::TRANSMISSION) {
                    RayType::Refraction
                } else {
                    panic!("Unimplemented path ray type {:?}", sample_type);
                };
            } else {
                // TODO: pbrt doesn't do this on miss after first ray in path,
                //       but on direct illumination estimate for previous hit
                incoming_radiance += beta * scene.background;
                break;
            };

            // Russian roulette
            if bounces > 3 {
                let q = (1.0 - beta.g).max(0.05);
                if sampler.get_1d() < q {
                    break;
                }
                beta *= Spectrum::ones() / (1.0 - q);
            }

            bounces += 1;
        }

        RadianceResult {
            li: incoming_radiance,
            ray_scene_intersections: ray_count,
            rays: collected_rays,
        }
    }
}
