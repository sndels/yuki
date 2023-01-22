use glium::Surface;

pub struct ScaleOutput {
    vertex_buffer: glium::VertexBuffer<Vertex>,
    index_buffer: glium::IndexBuffer<u16>,
    program: glium::Program,
}

impl ScaleOutput {
    pub fn new<T: glium::backend::Facade>(backend: &T) -> Result<Self, NewError> {
        // Positions only to mark the relative positions, actual positions are set from uniforms using
        let vertex_buffer = glium::VertexBuffer::new(
            backend,
            &[
                Vertex {
                    position: [-1.0, -1.0],
                    uv: [0.0, 0.0],
                },
                Vertex {
                    position: [1.0, -1.0],
                    uv: [1.0, 0.0],
                },
                Vertex {
                    position: [1.0, 1.0],
                    uv: [1.0, 1.0],
                },
                Vertex {
                    position: [-1.0, 1.0],
                    uv: [0.0, 1.0],
                },
            ],
        )
        .map_err(NewError::VertexBuffer)?;

        let index_buffer = glium::IndexBuffer::new(
            backend,
            glium::index::PrimitiveType::TrianglesList,
            &[0_u16, 1, 2, 0, 2, 3],
        )
        .map_err(NewError::IndexBuffer)?;

        let program = glium::Program::from_source(backend, VS_CODE, FS_CODE, None)
            .map_err(NewError::Program)?;

        Ok(Self {
            vertex_buffer,
            index_buffer,
            program,
        })
    }

    pub fn draw(
        &self,
        texture: &glium::Texture2d,
        frame: &mut glium::Frame,
    ) -> Result<(), glium::DrawError> {
        let input_sampler = texture
            .sampled()
            .wrap_function(glium::uniforms::SamplerWrapFunction::BorderClamp)
            .minify_filter(glium::uniforms::MinifySamplerFilter::Linear)
            .magnify_filter(glium::uniforms::MagnifySamplerFilter::Linear);

        // Retain film aspect ratio, flip y since we have 0,0 at top left and gl at bottom left
        let (width, height) = frame.get_dimensions();
        let frame_aspect = (width as f32) / (height as f32);
        let texture_aspect = (texture.width() as f32) / (texture.height() as f32);
        let target_rect = if frame_aspect < texture_aspect {
            let scaled_height = (width * texture.height()) / texture.width();
            glium::BlitTarget {
                left: 0,
                bottom: (height.saturating_sub(scaled_height) / 2) + scaled_height,
                width: i32::try_from(width).unwrap(),
                height: -i32::try_from(scaled_height).unwrap(),
            }
        } else {
            let scaled_width = (height * texture.width()) / texture.height();
            glium::BlitTarget {
                left: width.saturating_sub(scaled_width) / 2,
                bottom: height,
                width: i32::try_from(scaled_width).unwrap(),
                height: -i32::try_from(height).unwrap(),
            }
        };

        let left_ndc = (target_rect.left as f32) / (width as f32) * 2.0 - 1.0;
        let bottom_ndc = (target_rect.bottom as f32) / (height as f32) * 2.0 - 1.0;
        let right_ndc = left_ndc + 2.0 * (target_rect.width as f32) / (width as f32);
        let top_ndc = bottom_ndc + 2.0 * (target_rect.height as f32) / (height as f32);

        let uniforms = glium::uniform! {
            input_texture: input_sampler,
            left: left_ndc,
            bottom: bottom_ndc,
            right: right_ndc,
            top: top_ndc,
        };

        frame.draw(
            &self.vertex_buffer,
            &self.index_buffer,
            &self.program,
            &uniforms,
            &glium::DrawParameters::default(),
        )
    }
}

#[derive(Copy, Clone)]
struct Vertex {
    position: [f32; 2],
    uv: [f32; 2],
}
glium::implement_vertex!(Vertex, position, uv);

const VS_CODE: &str = r#"
#version 410 core

uniform float left;
uniform float bottom;
uniform float right;
uniform float top;

in vec2 position;
in vec2 uv;

out vec2 frag_uv;

void main() {
    frag_uv = uv;
    if (gl_VertexID == 0)
        gl_Position = vec4(left, bottom, 0, 1);
    else if (gl_VertexID == 1)
        gl_Position = vec4(right, bottom, 0, 1);
    else if (gl_VertexID == 2)
        gl_Position = vec4(right, top, 0, 1);
    else
        gl_Position = vec4(left, top, 0, 1);
}
"#;

const FS_CODE: &str = r#"
#version 410 core

uniform sampler2D input_texture;

in vec2 frag_uv;

out vec4 output_color;

void main() {
    vec3 color = texture(input_texture, frag_uv).rgb;

    output_color = vec4(color, 1);
}
"#;

#[derive(Debug)]
pub enum NewError {
    VertexBuffer(glium::vertex::BufferCreationError),
    IndexBuffer(glium::index::BufferCreationError),
    Program(glium::ProgramCreationError),
}
