use crate::{
    math::{
        point::Point3,
        transform::{translation, Transform},
        vector::Vec3,
    },
    point_light::PointLight,
    shapes::{mesh::Mesh, shape::Shape, sphere::Sphere, triangle::Triangle},
};

use std::sync::Arc;

pub struct Scene {
    pub meshes: Vec<Arc<Mesh>>,
    pub geometry: Arc<Vec<Box<dyn Shape>>>,
    pub light: Arc<PointLight>,
    pub cam_pos: Point3<f32>,
    pub cam_target: Point3<f32>,
    pub cam_fov: f32,
}

impl Scene {
    /// The cornell box with a tall box and a sphere
    /// Lifted from http://www.graphics.cornell.edu/online/box/data.html
    pub fn cornell() -> Scene {
        // Original uses a right-handed coordinate system so flip z
        let handedness_swap = Transform::new([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, -1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]);

        // Materials
        // These are approximate as the originals are defined as spectrums
        let white = Vec3::from(180.0) / 255.0;
        let red = Vec3::new(180.0, 0.0, 0.0) / 255.0;
        let green = Vec3::new(0.0, 180.0, 0.0) / 255.0;

        let mut meshes: Vec<Arc<Mesh>> = Vec::new();
        let mut geometry: Vec<Box<dyn Shape>> = Vec::new();

        // Walls
        {
            let wall_meshes = vec![
                // Floor
                Arc::new(Mesh::new(
                    &handedness_swap,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(0.0, 0.0, 0.0),
                        Point3::new(552.8, 0.0, 0.0),
                        Point3::new(549.6, 0.0, 559.2),
                        Point3::new(0.0, 0.0, 559.2),
                    ],
                )),
                // Ceiling
                Arc::new(Mesh::new(
                    &handedness_swap,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(0.0, 548.8, 0.0),
                        Point3::new(556.0, 548.8, 0.0),
                        Point3::new(556.0, 548.8, 559.2),
                        Point3::new(0.0, 548.8, 559.2),
                    ],
                )),
                // Back wall
                Arc::new(Mesh::new(
                    &handedness_swap,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(0.0, 0.0, 559.2),
                        Point3::new(549.6, 0.0, 559.2),
                        Point3::new(556.0, 548.8, 559.2),
                        Point3::new(0.0, 548.8, 559.2),
                    ],
                )),
                // Right wall
                Arc::new(Mesh::new(
                    &handedness_swap,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(0.0, 0.0, 0.0),
                        Point3::new(0.0, 0.0, 559.2),
                        Point3::new(0.0, 548.8, 559.2),
                        Point3::new(0.0, 548.8, 0.0),
                    ],
                )),
                // Left wall
                Arc::new(Mesh::new(
                    &handedness_swap,
                    vec![0, 1, 2, 0, 2, 3],
                    vec![
                        Point3::new(552.8, 0.0, 0.0),
                        Point3::new(549.6, 0.0, 559.2),
                        Point3::new(556.0, 548.8, 559.2),
                        Point3::new(556.0, 548.8, 0.0),
                    ],
                )),
            ];

            let materials = [white, white, white, green, red];
            for (mesh, material) in wall_meshes.iter().zip(materials.iter()) {
                for v0 in (0..mesh.indices.len()).step_by(3) {
                    geometry.push(Box::new(Triangle::new(mesh.clone(), v0, *material)));
                }
            }
            meshes.extend(wall_meshes);
        }

        // Tall box
        {
            let mesh = Arc::new(Mesh::new(
                &handedness_swap,
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
            ));

            for v0 in (0..mesh.indices.len()).step_by(3) {
                geometry.push(Box::new(Triangle::new(mesh.clone(), v0, white)));
            }
            meshes.push(mesh);
        }

        geometry.push(Box::new(Sphere::new(
            &translation(Vec3::new(186.0, 82.5, -168.5)),
            82.5,
            white,
        )));

        let light = Arc::new(PointLight::new(
            &translation(Vec3::new(288.0, 547.0, -279.0)),
            Vec3::from(60000.0),
        ));

        let cam_pos = Point3::new(278.0, 273.0, 800.0);
        let cam_target = Point3::new(278.0, 273.0, -260.0);
        let cam_fov = 40.0;

        Scene {
            meshes,
            geometry: Arc::new(geometry),
            light,
            cam_pos,
            cam_target,
            cam_fov,
        }
    }
}
