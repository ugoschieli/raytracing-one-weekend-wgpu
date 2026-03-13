use std::sync::Arc;

use winit::window::Window;

use crate::{
    raytracing::{RaytracingPass, RenderPass},
    utils::*,
};

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    pub egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    egui_textures_to_free: Vec<egui::TextureId>,
}

#[derive(Debug)]
pub struct Texture {
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Self {
        let instance = create_instance(None);
        let adapter = create_adapter(&instance, None);

        let (device, queue) = create_device(
            &adapter,
            Some(wgpu::DeviceDescriptor {
                required_features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                ..Default::default()
            }),
        );
        let surface = create_surface(&instance, window.clone());
        let surface_config =
            configure_surface(&surface, &device, &adapter, window.inner_size(), None);

        let context = egui::Context::default();
        let viewport_id = context.viewport_id();
        let egui_state = egui_winit::State::new(
            context,
            viewport_id,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            surface_config.format,
            egui_wgpu::RendererOptions {
                ..Default::default()
            },
        );

        Self {
            device,
            queue,
            surface,
            surface_config,
            egui_state,
            egui_renderer,
            egui_textures_to_free: Vec::new(),
        }
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn surface(&self) -> &wgpu::Surface<'static> {
        &self.surface
    }

    pub fn surface_config(&self) -> &wgpu::SurfaceConfiguration {
        &self.surface_config
    }

    pub fn create_texture_2d(
        &self,
        label: &str,
        width: u32,
        height: u32,
        usage: wgpu::TextureUsages,
        format: wgpu::TextureFormat,
    ) -> Texture {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            label: Some(label),
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some(format!("{} view", label).as_str()),
            ..Default::default()
        });

        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            label: Some(format!("{} sampler", label).as_str()),
            ..Default::default()
        });

        Texture { view, sampler }
    }

    pub fn render(
        &mut self,
        window: &Window,
        raytracing_pass: &RaytracingPass,
        display_pass: &RenderPass,
        ui: impl FnMut(&egui::Context),
    ) -> Result<(), Box<dyn std::error::Error>> {
        let output = match self.surface().get_current_texture() {
            Ok(surface) => surface,
            Err(wgpu::SurfaceError::Outdated) => {
                log::warn!("Surface is outdated");
                return Ok(());
            }
            Err(e) => return Err(Box::new(e)),
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // We only use one encoder for everything to avoid lifetime complexites
        let mut encoder = self
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        let (clipped_primitives, screen_descriptor) = self.update_ui(window, &mut encoder, ui);

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });

            raytracing_pass.compute(self, &mut compute_pass);
        }

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            display_pass.render(&mut render_pass);
            self.render_ui(render_pass, &clipped_primitives, &screen_descriptor);
        }

        self.queue().submit(Some(encoder.finish()));
        output.present();

        self.cleanup_ui();

        Ok(())
    }

    pub fn update_ui(
        &mut self,
        window: &Window,
        encoder: &mut wgpu::CommandEncoder,
        ui: impl FnMut(&egui::Context),
    ) -> (Vec<egui::ClippedPrimitive>, egui_wgpu::ScreenDescriptor) {
        let raw_input = self.egui_state.take_egui_input(window);

        let full_output = self.egui_state.egui_ctx().run(raw_input, ui);

        let clipped_primitives = self
            .egui_state
            .egui_ctx()
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.surface_config().width, self.surface_config().height],
            pixels_per_point: window.scale_factor() as f32,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }

        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        self.egui_textures_to_free
            .extend(full_output.textures_delta.free);

        (clipped_primitives, screen_descriptor)
    }

    pub fn render_ui<'a>(
        &'a self,
        render_pass: wgpu::RenderPass<'a>,
        clipped_primitives: &[egui::ClippedPrimitive],
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
    ) {
        self.egui_renderer.render(
            &mut render_pass.forget_lifetime(),
            clipped_primitives,
            screen_descriptor,
        );
    }

    pub fn cleanup_ui(&mut self) {
        for x in self.egui_textures_to_free.drain(..) {
            self.egui_renderer.free_texture(&x);
        }
    }
}
