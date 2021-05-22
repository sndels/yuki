use glium::Surface;

pub struct ScaleOutput {}

impl ScaleOutput {
    pub fn draw(texture: &glium::Texture2d, frame: &mut glium::Frame) {
        let source_rect = glium::Rect {
            left: 0,
            bottom: 0,
            width: texture.width(),
            height: texture.height(),
        };

        // Retain film aspect ratio, flip y since we have 0,0 at top left and gl at bottom left
        let (width, height) = frame.get_dimensions();
        let frame_aspect = (width as f32) / (height as f32);
        let texture_aspect = (texture.width() as f32) / (texture.height() as f32);
        let target_rect = if frame_aspect < texture_aspect {
            let scaled_height = (width * texture.height()) / texture.width();
            glium::BlitTarget {
                left: 0,
                bottom: (height.saturating_sub(scaled_height) / 2) + scaled_height,
                width: width as i32,
                height: -(scaled_height as i32),
            }
        } else {
            let scaled_width = (height * texture.width()) / texture.height();
            glium::BlitTarget {
                left: width.saturating_sub(scaled_width) / 2,
                bottom: height,
                width: scaled_width as i32,
                height: -(height as i32),
            }
        };

        texture.as_surface().blit_color(
            &source_rect,
            frame,
            &target_rect,
            glium::uniforms::MagnifySamplerFilter::Linear,
        );
    }
}
