use crate::{
    film::FilmSettings,
    math::{
        transforms::{look_at, scale, translation},
        Point2, Point3, Ray, Transform, Vec2, Vec3,
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

#[derive(Copy, Clone)]
pub struct CameraParameters {
    pub position: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Vec3<f32>,
    pub fov: FoV,
}

impl Default for CameraParameters {
    fn default() -> Self {
        Self {
            position: Point3::new(0.0, 0.0, 0.0),
            target: Point3::new(0.0, 0.0, 0.0),
            up: Vec3::new(0.0, 1.0, 0.0),
            fov: FoV::X(0.0),
        }
    }
}

// Angle in degrees
#[derive(Copy, Clone)]
pub enum FoV {
    X(f32),
    Y(f32),
}

impl Camera {
    /// Creates a new `Camera`. `fov` is horizontal and in degrees.
    pub fn new(params: CameraParameters, film_settings: FilmSettings) -> Self {
        let camera_to_world = look_at(params.position, params.target, params.up).inverted();
        // Standard perspective projection with aspect ratio
        // Screen is
        // NOTE: pbrt uses a 1:1 image plane with a cutout region
        //       that could be nice for debugging purposes, though ui requires some thought
        // We don't really care about near, far since we only use this to project rays
        let near = 1e-2;
        let far = 1000.0;
        let fov_angle = match params.fov {
            FoV::X(v) | FoV::Y(v) => v,
        };
        let inv_tan = 1.0 / ((fov_angle.to_radians() / 2.0).tan());
        let camera_to_screen = &scale(inv_tan, inv_tan, 1.0)
            * &Transform::new([
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, far / (far - near), -(far * near) / (far - near)],
                [0.0, 0.0, 1.0, 0.0],
            ]);

        // Screen window
        // pbrt default is [-1,1] along the shorter axis and proportionally scaled on the other
        // We adapt the mitsuba convention that has a directional fov by scaling that to 1
        let film_x = film_settings.res.x as f32;
        let film_y = film_settings.res.y as f32;
        let (screen_min, screen_max) = match params.fov {
            FoV::X(_) => {
                let ar = film_x / film_y;
                (Vec2::new(-1.0, -1.0 / ar), Vec2::new(1.0, 1.0 / ar))
            }
            FoV::Y(_) => {
                let ar = film_y / film_x;
                (Vec2::new(-1.0 / ar, -1.0), Vec2::new(1.0 / ar, 1.0))
            }
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
            camera_to_world,
            raster_to_camera,
        }
    }

    /// Creates a new [Ray] at the camera sample with this `Camera`.
    pub fn ray(&self, sample: &CameraSample) -> Ray<f32> {
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
