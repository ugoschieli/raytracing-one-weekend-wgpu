use std::sync::Arc;

use glam::Vec3;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::platform::macos::WindowAttributesExtMacOS;
use winit::window::{Fullscreen, Window, WindowId};

use crate::camera::{Camera, CameraUniforms};
use crate::cube::{Cube, World};
use crate::rasterizer::RasterizerPass;
use crate::raytracing::{RaytracingPass, RenderPass as DisplayPass};
use crate::renderer::Renderer;

mod camera;
mod cube;
mod rasterizer;
mod raytracing;
mod renderer;
mod utils;

#[derive(Default)]
struct InputState {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    rmb_pressed: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Poll);
    let mut app = App::default();
    event_loop.run_app(&mut app)?;

    Ok(())
}

#[derive(Default)]
struct App {
    renderer: Option<Renderer>,
    rasterizing_pass: Option<RasterizerPass>,
    raytracing_pass: Option<RaytracingPass>,
    display_pass: Option<DisplayPass>,
    window: Option<Arc<Window>>,

    last_frame_time: Option<std::time::Instant>,
    fps: f32,
    frame_count: u32,

    camera: Camera,
    input_state: InputState,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    time: f32,
    frame: u32,
    _padding: [u32; 2],
    camera_uniforms: CameraUniforms,
}

impl App {
    fn get_window(&self) -> Arc<Window> {
        self.window.as_ref().unwrap().clone()
    }

    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let window = self.get_window();
        let renderer = Renderer::new(window.clone());

        let world = World::new(&[
            Cube::new(Vec3::new(0.0, 0.0, -10.0), Vec3::new(1.0, 0.0, 0.0), 0.5),
            Cube::new(Vec3::new(0.0, 1.0, -10.0), Vec3::new(1.0, 1.0, 0.0), 0.5),
            Cube::new(Vec3::new(1.0, 1.0, -10.0), Vec3::new(1.0, 0.0, 1.0), 0.5),
            Cube::new(Vec3::new(0.0, -101.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 100.0),
        ]);

        let rasterizing_pass = RasterizerPass::new(&renderer, &world);
        let raytracing_pass = RaytracingPass::new(
            &renderer,
            &world,
            &rasterizing_pass.albedo_texture,
            &rasterizing_pass.normal_texture,
            &rasterizing_pass.depth_texture,
        );
        let display_pass = DisplayPass::new(&renderer, raytracing_pass.texture());

        self.last_frame_time = Some(std::time::Instant::now());

        self.renderer = Some(renderer);
        self.rasterizing_pass = Some(rasterizing_pass);
        self.raytracing_pass = Some(raytracing_pass);
        self.display_pass = Some(display_pass);

        Ok(())
    }

    fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let renderer = self.renderer.as_mut().unwrap();
        let window = self.window.as_ref().unwrap();

        // FPS Calculation & Camera Update
        if let Some(last_time) = self.last_frame_time {
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(last_time).as_secs_f32();
            if elapsed > 0.0 {
                self.fps = 1.0 / elapsed;
                window.set_title(&format!("Ray Tracing - {:.1} FPS", self.fps));
            }
            self.last_frame_time = Some(now);

            let speed = 5.0 * elapsed;
            self.camera
                .update(&self.input_state, speed, &mut self.frame_count);
        }

        self.rasterizing_pass.as_ref().unwrap().update(
            renderer,
            &self.camera,
            &mut self.frame_count,
        );

        self.raytracing_pass.as_ref().unwrap().update(
            renderer,
            &self.camera,
            &mut self.frame_count,
        );

        renderer.render(
            window,
            self.rasterizing_pass.as_ref().unwrap(),
            self.raytracing_pass.as_ref().unwrap(),
            self.display_pass.as_ref().unwrap(),
        )?;

        Ok(())
    }
}

impl ApplicationHandler for App {
    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: winit::event::DeviceId,
        event: winit::event::DeviceEvent,
    ) {
        if let winit::event::DeviceEvent::MouseMotion { delta } = event {
            if self.input_state.rmb_pressed {
                self.camera.yaw += (delta.0 as f32) * 0.005;
                self.camera.pitch -= (delta.1 as f32) * 0.005;
                let clamp_val = 89.0f32.to_radians();
                self.camera.pitch = self.camera.pitch.clamp(-clamp_val, clamp_val);
                self.frame_count = 0;
            }
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.window = Some(Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("Ray Tracing")
                            .with_fullscreen(Some(Fullscreen::Borderless(None)))
                            .with_borderless_game(true),
                    )
                    .unwrap(),
            ));

            self.init().unwrap();
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    winit::event::KeyEvent {
                        physical_key: winit::keyboard::PhysicalKey::Code(key_code),
                        state,
                        ..
                    },
                ..
            } => {
                let is_pressed = state == winit::event::ElementState::Pressed;
                match key_code {
                    winit::keyboard::KeyCode::KeyW => self.input_state.forward = is_pressed,
                    winit::keyboard::KeyCode::KeyS => self.input_state.backward = is_pressed,
                    winit::keyboard::KeyCode::KeyA => self.input_state.left = is_pressed,
                    winit::keyboard::KeyCode::KeyD => self.input_state.right = is_pressed,
                    winit::keyboard::KeyCode::Space => self.input_state.up = is_pressed,
                    winit::keyboard::KeyCode::ShiftLeft => self.input_state.down = is_pressed,
                    _ => {}
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if button == winit::event::MouseButton::Right {
                    self.input_state.rmb_pressed = state == winit::event::ElementState::Pressed;
                }
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                match self.render() {
                    Ok(_) => {}
                    Err(e) => {
                        log::error!("{e}");
                        event_loop.exit();
                    }
                };
                self.window.as_ref().unwrap().request_redraw();
            }
            _ => {}
        }
    }
}
