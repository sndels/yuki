use crate::{
    film::FilmSettings,
    math::{
        point::{Point2, Point3},
        ray::Ray,
        transform::{scale, translation, Transform},
        vector::{Vec2, Vec3},
    },
};

// Based on Physically Based Rendering 3rd ed.
// http://www.pbr-book.org/3ed-2018/Camera_Models.html

/// Values needed to specify a camera ray
pub struct CameraSample {
    pub p_film: Point2<f32>,
}

/// A simple pinhole camera
#[derive(Clone)]
pub struct Camera {
    camera_to_world: Transform<f32>,
    raster_to_camera: Transform<f32>,
}

impl Camera {
    /// Creates a new `Camera`. `fov` is horizontal and in degrees.
    pub fn new(camera_to_world: &Transform<f32>, fov: f32, film_settings: &FilmSettings) -> Self {
        // Standard perspective projection with aspect ratio
        // Screen is
        // NOTE: pbrt uses a 1:1 image plane with a cutout region
        //       that could be nice for debugging purposes, though ui requires some thought
        // We don't really care about near, far since we only use this to project rays
        let near = 1e-2;
        let far = 1000.0;
        let inv_tan = 1.0 / ((fov.to_radians() / 2.0).tan());
        let camera_to_screen = &scale(inv_tan, inv_tan, 1.0)
            * &Transform::new([
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, far / (far - near), -(far * near) / (far - near)],
                [0.0, 0.0, 1.0, 0.0],
            ]);

        // Screen window, pbrt default is [-1,1] along the shorter axis and
        // proportionally scaled on the other
        let film_x = film_settings.res.x as f32;
        let film_y = film_settings.res.y as f32;
        let (screen_min, screen_max) = if film_x > film_y {
            let ar = film_x / film_y;
            (Vec2::new(-ar, -1.0), Vec2::new(ar, 1.0))
        } else {
            let ar = film_y / film_x;
            (Vec2::new(-1.0, -ar), Vec2::new(1.0, ar))
        };
        let screen_to_raster = &scale(film_x, film_y, 1.0)
            * &(&scale(
                1.0 / (screen_max.x - screen_min.x),
                1.0 / (screen_min.y - screen_max.y),
                1.0,
            ) * &translation(Vec3::new(-screen_min.x, -screen_max.y, 0.0)));

        let raster_to_screen = screen_to_raster.inverted();
        let raster_to_camera = &camera_to_screen.inverted() * &raster_to_screen;

        Self {
            camera_to_world: camera_to_world.clone(),
            raster_to_camera,
        }
    }

    /// Creates a new [Ray] at the camera sample with this `Camera`.
    pub fn ray(&self, sample: CameraSample) -> Ray<f32> {
        let p_film = Point3::new(sample.p_film.x, sample.p_film.y, 0.0);
        let p_camera = &self.raster_to_camera * p_film;
        let r = Ray::new(
            Point3::zeros(),
            Vec3::from(p_camera).normalized(),
            f32::INFINITY,
        );
        &self.camera_to_world * r
    }
}
