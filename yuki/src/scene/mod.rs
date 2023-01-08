mod mitsuba;
mod pbrt;
mod ply;

use crate::{
    bvh::{BoundingVolumeHierarchy, SplitMethod},
    camera::{CameraParameters, FoV},
    film::FilmSettings,
    lights::{AreaLight, Light, PointLight, RectangularLight},
    materials::{Glass, Material, Matte, Metal},
    math::{
        transforms::{scale, translation},
        Point2, Point3, Spectrum, Transform, Vec2, Vec3,
    },
    shapes::{Mesh, Shape, Sphere, Triangle},
    textures::{ConstantTexture, ImageTexture},
    yuki_info,
};
use ply::PlyResult;
use serde::{Deserialize, Serialize};

use std::{path::PathBuf, sync::Arc, time::Instant};

#[derive(Clone, Deserialize, Serialize)]
pub struct SceneLoadSettings {
    pub path: PathBuf,
    pub split_method: SplitMethod,
    pub max_shapes_in_node: u16,
}

impl Default for SceneLoadSettings {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            split_method: SplitMethod::SurfaceAreaHeuristic,
            max_shapes_in_node: 1,
        }
    }
}

pub struct Scene {
    pub name: String,
    pub load_settings: SceneLoadSettings,
    pub meshes: Vec<Arc<Mesh>>,
    pub shapes: Arc<Vec<Arc<dyn Shape>>>,
    pub bvh: BoundingVolumeHierarchy,
    pub lights: Vec<Arc<dyn Light>>,
    pub background: Spectrum<f32>,
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

impl Scene {
    /// Loads a pbrt-v3 scene
    ///
    /// Also returns the time it took to load in seconds.
    pub fn pbrt_v3(
        settings: &SceneLoadSettings,
    ) -> Result<(Scene, CameraParameters, FilmSettings, f32)> {
        let load_start = Instant::now();

        let (scene, dynamic_params, film_settings) = match pbrt::load(settings) {
            Ok(ret) => ret,
            Err(why) => {
                // TODO: Proper error type?
                return Err(format!("{:?}", why).into());
            }
        };

        let total_secs = load_start.elapsed().as_secs_f32();

        yuki_info!("pbrt-v3: Loading took {:.2}s in total", total_secs);

        Ok((scene, dynamic_params, film_settings, total_secs))
    }

    /// Loads a Mitsuba 2 scene
    ///
    /// Also returns the time it took to load in seconds.
    pub fn mitsuba(
        settings: &SceneLoadSettings,
    ) -> Result<(Scene, CameraParameters, FilmSettings, f32)> {
        let load_start = Instant::now();

        let (scene, dynamic_params, film_settings) = mitsuba::load(settings)?;

        let total_secs = load_start.elapsed().as_secs_f32();

        yuki_info!("Mitsuba 2.0: Loading took {:.2}s in total", total_secs);

        Ok((scene, dynamic_params, film_settings, total_secs))
    }

    ///
    /// Loads a PLY, scales it to fit 2 units around the origin and orients the camera
    /// on it at an angle.
    ///
    /// Also returns the time it took to load in seconds.
    pub fn ply(
        settings: &SceneLoadSettings,
    ) -> Result<(Scene, CameraParameters, FilmSettings, f32)> {
        let load_start = Instant::now();

        let white = Arc::new(Matte::new(
            Arc::new(ConstantTexture::new(Spectrum::ones())),
            Arc::new(ConstantTexture::new(0.0)),
        )) as Arc<dyn Material>;
        let PlyResult { mesh, shapes } = ply::load(&settings.path, &white, None)?;

        let meshes = vec![mesh];

        let (bvh, shapes) = BoundingVolumeHierarchy::new(
            shapes,
            settings.max_shapes_in_node as usize,
            settings.split_method,
        );

        let light = Arc::new(PointLight::new(
            &translation(Vec3::new(5.0, 5.0, 0.0)),
            Spectrum::ones() * 600.0,
        ));

        let cam_pos = Point3::new(2.0, 2.0, 2.0);
        let cam_target = Point3::new(0.0, 0.0, 0.0);
        let cam_fov = FoV::X(40.0);

        let total_secs = load_start.elapsed().as_secs_f32();

        yuki_info!("PLY: Loading took {:.2}s in total", total_secs);

        Ok((
            Self {
                name: settings.path.file_name().unwrap().to_str().unwrap().into(),
                load_settings: settings.clone(),
                meshes,
                shapes,
                bvh,
                lights: vec![light],
                background: Spectrum::zeros(),
            },
            CameraParameters {
                position: cam_pos,
                target: cam_target,
                fov: cam_fov,
                ..CameraParameters::default()
            },
            FilmSettings::default(),
            total_secs,
        ))
    }

    /// Constructs the Cornell box holding a tall box and a sphere
    // Lifted from http://www.graphics.cornell.edu/online/box/data.html
    pub fn cornell() -> (Arc<Scene>, CameraParameters, FilmSettings, f32) {
        // These are close to the real measurements, but let's align everything
        // for simplicity
        const LEFT: f32 = 555.0;
        const RIGHT: f32 = 0.0;
        const X_CENTER: f32 = (LEFT + RIGHT) / 2.0;
        const BOTTOM: f32 = 0.0;
        const TOP: f32 = 550.0;
        const FRONT: f32 = 0.0;
        const BACK: f32 = 560.0;
        const Z_CENTER: f32 = (FRONT + BACK) / 2.0;
        const HEIGHT: f32 = TOP - BOTTOM;
        const LIGHT_WH: f32 = 100.0;
        const LIGHT_HALF_WH: f32 = LIGHT_WH / 2.0;
        const LIGHT_FRONT: f32 = Z_CENTER - LIGHT_HALF_WH;
        const LIGHT_BACK: f32 = Z_CENTER + LIGHT_HALF_WH;
        const LIGHT_LEFT: f32 = X_CENTER + LIGHT_HALF_WH;
        const LIGHT_RIGHT: f32 = X_CENTER - LIGHT_HALF_WH;
        const HOLE_TOP: f32 = TOP + HEIGHT * 0.025;

        let load_start = Instant::now();

        // Original uses a right-handed coordinate system so flip z
        let handedness_swap = Transform::new([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);

        let into_meters = scale(0.001, 0.001, 0.001);
        let handedness_swap_and_into_meters = &into_meters * &handedness_swap;

        // Materials
        // These are approximate as the originals are defined as spectrums
        let white = Arc::new(Matte::new(
            Arc::new(ConstantTexture::new(Spectrum::ones() * 180.0 / 255.0)),
            Arc::new(ConstantTexture::new(0.0)),
        ));
        let image = Arc::new(Matte::new(
            Arc::new(
                ImageTexture::from_image_bytes(std::include_bytes!(
                    "../../../res/tiling_58-1K/tiling_58_basecolor-1K.png"
                ))
                .unwrap(),
            ),
            Arc::new(ConstantTexture::new(0.0)),
        ));
        let red = Arc::new(Matte::new(
            Arc::new(ConstantTexture::new(Spectrum::new(180.0, 0.0, 0.0) / 255.0)),
            Arc::new(ConstantTexture::new(0.0)),
        ));
        let green = Arc::new(Matte::new(
            Arc::new(ConstantTexture::new(Spectrum::new(0.0, 180.0, 0.0) / 255.0)),
            Arc::new(ConstantTexture::new(0.0)),
        ));
        let blackbody = Arc::new(Matte::new(
            Arc::new(ConstantTexture::new(Spectrum::zeros())),
            Arc::new(ConstantTexture::new(0.0)),
        ));
        let copper = Arc::new(Metal::new(
            Arc::new(ConstantTexture::new(Spectrum::new(
                0.271_05, 0.676_93, 1.316_40,
            ))),
            Arc::new(ConstantTexture::new(Spectrum::new(
                3.60920, 2.62480, 2.29210,
            ))),
            Arc::new(ConstantTexture::new(0.01)),
            true,
        ));
        let glass = Arc::new(Glass::new(
            Arc::new(ConstantTexture::new(Spectrum::ones())),
            Arc::new(ConstantTexture::new(Spectrum::ones())),
            1.5,
        ));

        let light = {
            let size = Vec2::new(LIGHT_WH, LIGHT_WH) / 1000.0;
            let area = size.x * size.y;
            let power = 2.0;
            let radiance = power / (area * std::f32::consts::PI);
            Arc::new(RectangularLight::new(
                &translation(Vec3::new(X_CENTER, HOLE_TOP, -Z_CENTER) / 1000.0),
                Spectrum::ones() * radiance,
                size,
            ))
        };

        let mut meshes: Vec<Arc<Mesh>> = Vec::new();
        let mut shapes: Vec<Arc<dyn Shape>> = Vec::new();

        // Light
        {
            let mesh = Arc::new(Mesh::new(
                &handedness_swap_and_into_meters,
                vec![0, 1, 2, 0, 2, 3],
                vec![
                    Point3::new(LIGHT_RIGHT, HOLE_TOP, LIGHT_FRONT),
                    Point3::new(LIGHT_LEFT, HOLE_TOP, LIGHT_FRONT),
                    Point3::new(LIGHT_LEFT, HOLE_TOP, LIGHT_BACK),
                    Point3::new(LIGHT_RIGHT, HOLE_TOP, LIGHT_BACK),
                ],
                Vec::new(),
                Vec::new(),
            ));
            for v0 in (0..mesh.indices.len()).step_by(3) {
                shapes.push(Arc::new(Triangle::new(
                    Arc::clone(&mesh),
                    v0,
                    Arc::clone(&blackbody) as Arc<dyn Material>,
                    Some(Arc::clone(&light) as Arc<dyn AreaLight>),
                )));
            }
            meshes.push(mesh);
        }

        // Walls
        {
            let wall_meshes = vec![
                // Floor
                Arc::new(Mesh::new(
                    &handedness_swap_and_into_meters,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(RIGHT, BOTTOM, BACK),
                        Point3::new(LEFT, BOTTOM, BACK),
                        Point3::new(LEFT, BOTTOM, FRONT),
                        Point3::new(RIGHT, BOTTOM, FRONT),
                    ],
                    Vec::new(),
                    Vec::new(),
                )),
                // Ceiling, front
                Arc::new(Mesh::new(
                    &handedness_swap_and_into_meters,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(RIGHT, TOP, FRONT),
                        Point3::new(LEFT, TOP, FRONT),
                        Point3::new(LEFT, TOP, LIGHT_FRONT),
                        Point3::new(RIGHT, TOP, LIGHT_FRONT),
                    ],
                    Vec::new(),
                    Vec::new(),
                )),
                // Ceiling, back
                Arc::new(Mesh::new(
                    &handedness_swap_and_into_meters,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(RIGHT, TOP, LIGHT_BACK),
                        Point3::new(LEFT, TOP, LIGHT_BACK),
                        Point3::new(LEFT, TOP, BACK),
                        Point3::new(RIGHT, TOP, BACK),
                    ],
                    Vec::new(),
                    Vec::new(),
                )),
                // Ceiling, left
                Arc::new(Mesh::new(
                    &handedness_swap_and_into_meters,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(LIGHT_LEFT, TOP, FRONT),
                        Point3::new(LEFT, TOP, FRONT),
                        Point3::new(LEFT, TOP, BACK),
                        Point3::new(LIGHT_LEFT, TOP, BACK),
                    ],
                    Vec::new(),
                    Vec::new(),
                )),
                // Ceiling, right
                Arc::new(Mesh::new(
                    &handedness_swap_and_into_meters,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(RIGHT, TOP, FRONT),
                        Point3::new(LIGHT_RIGHT, TOP, FRONT),
                        Point3::new(LIGHT_RIGHT, TOP, BACK),
                        Point3::new(RIGHT, TOP, BACK),
                    ],
                    Vec::new(),
                    Vec::new(),
                )),
                // Hole, front
                Arc::new(Mesh::new(
                    &handedness_swap_and_into_meters,
                    vec![0, 2, 1, 0, 3, 2],
                    vec![
                        Point3::new(LIGHT_RIGHT, HOLE_TOP, LIGHT_FRONT),
                        Point3::new(LIGHT_LEFT, HOLE_TOP, LIGHT_FRONT),
                        Point3::new(LIGHT_LEFT, TOP, LIGHT_FRONT),
                        Point3::new(LIGHT_RIGHT, TOP, LIGHT_FRONT),
                    ],
                    Vec::new(),
                    Vec::new(),
                )),
                // Hole, front
                Arc::new(Mesh::new(
                    &handedness_swap_and_into_meters,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(LIGHT_RIGHT, HOLE_TOP, LIGHT_BACK),
                        Point3::new(LIGHT_LEFT, HOLE_TOP, LIGHT_BACK),
                        Point3::new(LIGHT_LEFT, TOP, LIGHT_BACK),
                        Point3::new(LIGHT_RIGHT, TOP, LIGHT_BACK),
                    ],
                    Vec::new(),
                    Vec::new(),
                )),
                // Hole, left
                Arc::new(Mesh::new(
                    &handedness_swap_and_into_meters,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(LIGHT_LEFT, TOP, LIGHT_FRONT),
                        Point3::new(LIGHT_LEFT, TOP, LIGHT_BACK),
                        Point3::new(LIGHT_LEFT, HOLE_TOP, LIGHT_BACK),
                        Point3::new(LIGHT_LEFT, HOLE_TOP, LIGHT_FRONT),
                    ],
                    Vec::new(),
                    Vec::new(),
                )),
                // Hole, right
                Arc::new(Mesh::new(
                    &handedness_swap_and_into_meters,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(LIGHT_RIGHT, HOLE_TOP, LIGHT_FRONT),
                        Point3::new(LIGHT_RIGHT, HOLE_TOP, LIGHT_BACK),
                        Point3::new(LIGHT_RIGHT, TOP, LIGHT_BACK),
                        Point3::new(LIGHT_RIGHT, TOP, LIGHT_FRONT),
                    ],
                    Vec::new(),
                    Vec::new(),
                )),
                // Back wall
                Arc::new(Mesh::new(
                    &handedness_swap_and_into_meters,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(RIGHT, TOP, BACK),
                        Point3::new(LEFT, TOP, BACK),
                        Point3::new(LEFT, BOTTOM, BACK),
                        Point3::new(RIGHT, BOTTOM, BACK),
                    ],
                    Vec::new(),
                    vec![
                        Point2::new(0.0, 0.0),
                        Point2::new(0.0, 1.0),
                        Point2::new(1.0, 1.0),
                        Point2::new(1.0, 0.0),
                    ],
                )),
                // Right wall
                Arc::new(Mesh::new(
                    &handedness_swap_and_into_meters,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(RIGHT, TOP, FRONT),
                        Point3::new(RIGHT, TOP, BACK),
                        Point3::new(RIGHT, BOTTOM, BACK),
                        Point3::new(RIGHT, BOTTOM, FRONT),
                    ],
                    Vec::new(),
                    Vec::new(),
                )),
                // Left wall
                Arc::new(Mesh::new(
                    &handedness_swap_and_into_meters,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(LEFT, BOTTOM, FRONT),
                        Point3::new(LEFT, BOTTOM, BACK),
                        Point3::new(LEFT, TOP, BACK),
                        Point3::new(LEFT, TOP, FRONT),
                    ],
                    Vec::new(),
                    Vec::new(),
                )),
            ];

            let materials = [
                Arc::clone(&white),
                Arc::clone(&white),
                Arc::clone(&white),
                Arc::clone(&white),
                Arc::clone(&white),
                Arc::clone(&white),
                Arc::clone(&white),
                Arc::clone(&white),
                Arc::clone(&white),
                image,
                green,
                red,
            ];
            assert!(materials.len() == wall_meshes.len());
            for (mesh, material) in wall_meshes.iter().zip(materials.iter()) {
                for v0 in (0..mesh.indices.len()).step_by(3) {
                    shapes.push(Arc::new(Triangle::new(
                        Arc::clone(mesh),
                        v0,
                        Arc::<Matte>::clone(material),
                        None,
                    )));
                }
            }
            meshes.extend(wall_meshes);
        }

        // Tall box
        {
            let mesh = Arc::new(Mesh::new(
                &handedness_swap_and_into_meters,
                vec![
                    0, 1, 2, 0, 2, 3, 4, 0, 3, 4, 3, 5, 5, 3, 2, 5, 2, 6, 6, 2, 1, 6, 1, 7, 7, 1,
                    0, 7, 0, 4,
                ],
                vec![
                    Point3::new(423.0, 330.0, 247.0),
                    Point3::new(265.0, 330.0, 296.0),
                    Point3::new(314.0, 330.0, 456.0),
                    Point3::new(472.0, 330.0, 406.0),
                    Point3::new(423.0, 0.0, 247.0),
                    Point3::new(472.0, 0.0, 406.0),
                    Point3::new(314.0, 0.0, 456.0),
                    Point3::new(265.0, 0.0, 296.0),
                ],
                Vec::new(),
                Vec::new(),
            ));

            for v0 in (0..mesh.indices.len()).step_by(3) {
                shapes.push(Arc::new(Triangle::new(
                    Arc::clone(&mesh),
                    v0,
                    Arc::<Glass>::clone(&glass),
                    None,
                )));
            }
            meshes.push(mesh);
        }

        shapes.push(Arc::new(Sphere::new(
            &translation(Vec3::new(0.186, 0.082, -0.168)),
            0.082,
            copper,
        )));

        let (bvh, shapes) = BoundingVolumeHierarchy::new(shapes, 1, SplitMethod::Middle);

        let cam_pos = Point3::new(0.278, 0.273, 0.800);
        let cam_target = Point3::new(0.278, 0.273, -0.260);
        let cam_fov = FoV::X(40.0);

        let total_secs = load_start.elapsed().as_secs_f32();

        (
            Arc::new(Scene {
                name: "Cornell Box".into(),
                load_settings: SceneLoadSettings::default(),
                meshes,
                shapes,
                bvh,
                lights: vec![light],
                background: Spectrum::zeros(),
            }),
            CameraParameters {
                position: cam_pos,
                target: cam_target,
                fov: cam_fov,
                ..CameraParameters::default()
            },
            FilmSettings::default(),
            total_secs,
        )
    }
}
