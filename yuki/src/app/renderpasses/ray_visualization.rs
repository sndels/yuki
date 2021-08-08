use glium::Surface;

use crate::{
    camera::{CameraParameters, FoV},
    film::FilmSettings,
    integrators::{IntegratorRay, RayType},
    math::{transforms::look_at, Bounds3, Matrix4x4, Point3, Transform, Vec3},
    yuki_trace,
};

pub struct RayVisualization {
    buffers: Option<(glium::VertexBuffer<Vertex>, glium::IndexBuffer<u16>)>,
    program: glium::Program,
}

impl RayVisualization {
    pub fn new<T: glium::backend::Facade>(
        backend: &T,
    ) -> Result<Self, glium::ProgramCreationError> {
        let program = glium::Program::from_source(backend, VS_CODE, FS_CODE, None)?;

        Ok(Self {
            buffers: None,
            program,
        })
    }

    pub fn set_rays<T: glium::backend::Facade>(
        &mut self,
        backend: &T,
        rays: &[IntegratorRay],
    ) -> Result<(), SetRaysError> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        for (i, IntegratorRay { ray, ray_type }) in rays.iter().enumerate() {
            let color = match ray_type {
                RayType::Direct => [1.0, 1.0, 1.0],
                RayType::Reflection => [1.0, 0.0, 0.0],
                RayType::Refraction => [0.0, 1.0, 0.0],
            };
            let p0 = ray.o;
            let p1 = ray.o + ray.d.normalized() * ray.t_max;
            vertices.push(Vertex {
                position: [p0.x, p0.y, p0.z],
                color,
            });
            vertices.push(Vertex {
                position: [p1.x, p1.y, p1.z],
                color,
            });
            indices.push((i * 2) as u16);
            indices.push((i * 2 + 1) as u16);
        }

        self.buffers = Some((
            glium::VertexBuffer::new(backend, &vertices).map_err(SetRaysError::VertexBuffer)?,
            glium::IndexBuffer::new(backend, glium::index::PrimitiveType::LinesList, &indices)
                .map_err(SetRaysError::IndexBuffer)?,
        ));

        Ok(())
    }

    pub fn clear_rays(&mut self) {
        self.buffers = None;
    }

    pub fn draw<'a>(
        &self,
        scene_bb: Bounds3<f32>,
        camera_params: CameraParameters,
        film_settings: FilmSettings,
        fb: &mut glium::framebuffer::SimpleFrameBuffer<'a>,
    ) -> Result<(), glium::DrawError> {
        if let Some((vbo, ibo)) = &self.buffers {
            yuki_trace!("draw: Buffers initialized, drawing.");

            let world_to_camera = look_at(
                camera_params.position,
                camera_params.target,
                Vec3::new(0.0, 1.0, 0.0),
            );

            let camera_to_clip = {
                let bb_points = {
                    let p0 = scene_bb.p_min;
                    let p1 = scene_bb.p_max;
                    vec![
                        p0,
                        Point3::new(p0.x, p0.y, p1.z),
                        Point3::new(p0.x, p1.y, p0.z),
                        Point3::new(p0.x, p1.y, p1.z),
                        Point3::new(p1.x, p0.y, p0.z),
                        Point3::new(p1.x, p0.y, p1.z),
                        Point3::new(p1.x, p1.y, p0.z),
                        p1,
                    ]
                };
                let zf = bb_points
                    .iter()
                    .fold(0.0, |acc, &p| (p - camera_params.position).len().max(acc));
                let zn = zf * 1e-5;

                let fov = match camera_params.fov {
                    FoV::X(angle) | FoV::Y(angle) => angle,
                };
                let tan_half_fov = (fov * 0.5).to_radians().tan();
                let (xf, yf) = match camera_params.fov {
                    FoV::X(_) => {
                        let ar = (film_settings.res.y as f32) / (film_settings.res.x as f32);
                        (1.0 / tan_half_fov, 1.0 / (tan_half_fov * ar))
                    }
                    FoV::Y(_) => {
                        let ar = (film_settings.res.x as f32) / (film_settings.res.y as f32);
                        (1.0 / (tan_half_fov * ar), 1.0 / tan_half_fov)
                    }
                };

                Transform::new_m(Matrix4x4::new([
                    [xf, 0.0, 0.0, 0.0],
                    [0.0, yf, 0.0, 0.0],
                    [
                        0.0,
                        0.0,
                        (zf + zn) / (zf - zn),
                        -(2.0 * zf * zn) / (zf - zn),
                    ],
                    [0.0, 0.0, 1.0, 0.0],
                ]))
            };

            // The film is "upside down" at this point
            let flip_y = Transform::new([
                [1.0, 0.0, 0.0, 0.0],
                [0.0, -1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ]);

            let trfn = &flip_y * &(&camera_to_clip * &world_to_camera);
            let m = trfn.m();

            let world_to_clip: [[f32; 4]; 4] = [
                [m.row(0)[0], m.row(1)[0], m.row(2)[0], m.row(3)[0]],
                [m.row(0)[1], m.row(1)[1], m.row(2)[1], m.row(3)[1]],
                [m.row(0)[2], m.row(1)[2], m.row(2)[2], m.row(3)[2]],
                [m.row(0)[3], m.row(1)[3], m.row(2)[3], m.row(3)[3]],
            ];

            let uniforms = glium::uniform! {
                world_to_clip: world_to_clip,
            };

            fb.draw(
                vbo,
                ibo,
                &self.program,
                &uniforms,
                &glium::DrawParameters::default(),
            )?;
        } else {
            yuki_trace!("draw: Buffers uninitialized, skipping.");
        }

        Ok(())
    }
}

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}
glium::implement_vertex!(Vertex, position, color);

const VS_CODE: &str = r#"
#version 410 core

uniform mat4 world_to_clip;

in vec3 position;
in vec3 color;

out vec3 frag_color;

void main() {
    frag_color = color;
    gl_Position = world_to_clip * vec4(position, 1);
}
"#;

const FS_CODE: &str = r#"
#version 410 core

uniform sampler2D input_texture;
uniform float exposure;

in vec3 frag_color;

out vec4 output_color;


void main() {
    output_color = vec4(frag_color, 1.0f);
}
"#;

#[derive(Debug)]
pub enum SetRaysError {
    VertexBuffer(glium::vertex::BufferCreationError),
    IndexBuffer(glium::index::BufferCreationError),
}
