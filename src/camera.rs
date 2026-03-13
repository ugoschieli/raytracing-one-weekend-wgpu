use crate::{InputState, renderer::Renderer};

pub struct Camera {
    pub pos: glam::Vec3,
    pub yaw: f32,
    pub pitch: f32,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniforms {
    pub center: glam::Vec3,
    _pad0: f32,
    pub pixel00_loc: glam::Vec3,
    _pad1: f32,
    pub pixel_delta_u: glam::Vec3,
    _pad2: f32,
    pub pixel_delta_v: glam::Vec3,
    _pad3: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            pos: glam::Vec3::new(0.0, 0.0, 0.0),
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: 0.0,
        }
    }
}

impl Camera {
    pub fn update(&mut self, input: &InputState, speed: f32, frame_count: &mut u32) {
        let (yaw_sin, yaw_cos) = self.yaw.sin_cos();
        let (pitch_sin, pitch_cos) = self.pitch.sin_cos();

        let forward =
            glam::Vec3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();

        let right = forward.cross(glam::Vec3::Y).normalize();

        let old_pos = self.pos;

        if input.forward {
            self.pos += forward * speed;
        }
        if input.backward {
            self.pos -= forward * speed;
        }
        if input.left {
            self.pos -= right * speed;
        }
        if input.right {
            self.pos += right * speed;
        }
        if input.up {
            self.pos += glam::Vec3::Y * speed;
        }
        if input.down {
            self.pos -= glam::Vec3::Y * speed;
        }

        if old_pos != self.pos {
            *frame_count = 0;
        }
    }

    pub fn update_uniforms(&self, renderer: &Renderer) -> CameraUniforms {
        let (yaw_sin, yaw_cos) = self.yaw.sin_cos();
        let (pitch_sin, pitch_cos) = self.pitch.sin_cos();
        let forward =
            glam::Vec3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin).normalize();
        let right = forward.cross(glam::Vec3::Y).normalize();
        let up = right.cross(forward).normalize();
        let width = renderer.surface_config().width as f32;
        let height = renderer.surface_config().height as f32;
        let aspect_ratio = width / height;

        let focal_length = 1.0;
        let viewport_height = 2.0;
        let viewport_width = viewport_height * aspect_ratio;
        let viewport_u = right * viewport_width;
        let viewport_v = -up * viewport_height;

        let pixel_delta_u = viewport_u / width;
        let pixel_delta_v = viewport_v / height;
        let viewport_upper_left =
            self.pos + (forward * focal_length) - viewport_u / 2.0 - viewport_v / 2.0;
        let pixel00_loc = viewport_upper_left + 0.5 * (pixel_delta_u + pixel_delta_v);

        CameraUniforms {
            center: self.pos,
            _pad0: 0.0,
            pixel00_loc,
            _pad1: 0.0,
            pixel_delta_u,
            _pad2: 0.0,
            pixel_delta_v,
            _pad3: 0.0,
        }
    }
}
