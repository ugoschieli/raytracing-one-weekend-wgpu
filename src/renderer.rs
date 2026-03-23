use std::sync::Arc;

use winit::window::Window;

use crate::{
    rasterizer::RasterizerPass,
    raytracing::{RaytracingPass, RenderPass as DisplayPass},
    utils::*,
};

pub struct Renderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
}

#[derive(Debug)]
pub struct Texture {
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Renderer {
    pub fn new(window: Arc<Window>) -> Self {
        let instance = create_wgpu_instance();
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

        Self {
            device,
            queue,
            surface,
            surface_config,
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
        _window: &Window,
        rasterizing_pass: &RasterizerPass,
        raytracing_pass: &RaytracingPass,
        display_pass: &DisplayPass,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let output = match self.surface().get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t)
            | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            _ => return Ok(()),
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

        rasterizing_pass.render(&mut encoder);

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Raytracing Compute Pass"),
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
                multiview_mask: None,
            });

            display_pass.render(&mut render_pass);
        }

        self.queue().submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }
}
