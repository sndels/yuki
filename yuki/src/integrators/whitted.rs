use super::{Integrator, IntegratorRay, RadianceResult, RayType};
use crate::{
    bvh::IntersectionResult,
    interaction::SurfaceInteraction,
    materials::{BxdfSample, BxdfType},
    math::{Ray, Vec3},
    scene::Scene,
    shapes::Hit,
};

#[derive(Copy, Clone)]
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
        ray_type: BxdfType,
        collect_rays: bool,
    ) -> RadianceResult {
        let BxdfSample { wi, f, sample_type } = si
            .bsdf
            .as_ref()
            .unwrap()
            .sample_f(si.wo, BxdfType::SPECULAR | ray_type);
        if sample_type == BxdfType::NONE {
            RadianceResult {
                li: Vec3::from(0.0),
                ray_scene_intersections: 0,
                rays: None,
            }
        } else {
            // TODO: Do color/spectrum class for this math
            fn mul(v1: Vec3<f32>, v2: Vec3<f32>) -> Vec3<f32> {
                Vec3::new(v1.x * v2.x, v1.y * v2.y, v1.z * v2.z)
            }

            let refl = si.spawn_ray(wi);

            let mut ret = self.li(refl, scene, depth + 1, collect_rays);
            ret.li = mul(f, ret.li) * wi.dot_n(si.n).abs();

            ret
        }
    }
}

impl Integrator for Whitted {
    fn li(&self, ray: Ray<f32>, scene: &Scene, depth: u32, collect_rays: bool) -> RadianceResult {
        let IntersectionResult { hit, .. } = scene.bvh.intersect(ray);

        let mut collected_rays: Option<Vec<IntegratorRay>> = None;
        let (incoming_radiance, ray_count) = if let Some(Hit { si, t, .. }) = hit {
            if collect_rays {
                collected_rays = Some(vec![IntegratorRay {
                    ray: Ray::new(ray.o, ray.d, t),
                    ray_type: RayType::Direct,
                }]);
            }

            // TODO: Do color/spectrum class for this math
            fn mul(v1: Vec3<f32>, v2: Vec3<f32>) -> Vec3<f32> {
                Vec3::new(v1.x * v2.x, v1.y * v2.y, v1.z * v2.z)
            }
            let mut ray_count = 1;
            let mut sum_li = scene.lights.iter().fold(Vec3::from(0.0), |c, l| {
                let light_sample = l.sample_li(&si);
                // TODO: Trace light visibility
                c + mul(
                    si.bsdf
                        .as_ref()
                        .unwrap()
                        .f(si.wo, light_sample.l, BxdfType::all()),
                    light_sample.li,
                ) * si.n.dot_v(light_sample.l).clamp(0.0, 1.0)
            });

            if depth + 1 < self.max_depth {
                macro_rules! spec {
                    ($t:expr, $rt:expr) => {
                        let specular_result =
                            self.specular_contribution(&si, scene, depth, $t, collect_rays);
                        sum_li += specular_result.li;
                        ray_count += specular_result.ray_scene_intersections;
                        if let Some(rays) = &mut collected_rays {
                            if let Some(mut spec_rays) = specular_result.rays {
                                spec_rays[0].ray_type = $rt;
                                rays.append(&mut spec_rays);
                            }
                        }
                    };
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
