use wgpu::util::DeviceExt;

use crate::{
    Uniforms,
    camera::Camera,
    renderer::{Renderer, Texture},
    utils::*,
};

#[derive(Debug)]
pub struct RaytracingPass {
    pub bind_group: wgpu::BindGroup,
    pub pipeline: wgpu::ComputePipeline,
    pub uniforms: wgpu::Buffer,
    pub raytracing_texture: Texture,
}

#[derive(Debug)]
pub struct RenderPass {
    pub bind_group: wgpu::BindGroup,
    pub pipeline: wgpu::RenderPipeline,
}

impl RaytracingPass {
    pub fn new(renderer: &Renderer, world: &crate::cube::World) -> Self {
        let device = renderer.device();
        let shader = device.create_shader_module(wgpu::include_wgsl!("raytracing.wgsl"));

        let raytracing_texture = renderer.create_texture_2d(
            "Raytracing Texture",
            renderer.surface_config().width,
            renderer.surface_config().height,
            wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::TextureFormat::Rgba8Unorm,
        );

        let accum_texture = renderer.create_texture_2d(
            "Accumulation Texture",
            renderer.surface_config().width,
            renderer.surface_config().height,
            wgpu::TextureUsages::STORAGE_BINDING,
            wgpu::TextureFormat::Rgba32Float,
        );

        let world_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("World Buffer"),
            contents: bytemuck::cast_slice(&world.to_uniform()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Time Buffer"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Raytracing Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::ReadWrite,
                        format: wgpu::TextureFormat::Rgba32Float,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Raytracing Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&raytracing_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniforms.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&accum_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: world_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline = create_compute_pipeline(
            device,
            "Raytracing Pipeline",
            &shader,
            &[Some(&bind_group_layout)],
        );

        Self {
            bind_group,
            pipeline,
            uniforms,
            raytracing_texture,
        }
    }

    pub fn texture(&self) -> &Texture {
        &self.raytracing_texture
    }

    pub fn update(&self, renderer: &Renderer, camera: &Camera, frame_count: &mut u32) {
        let camera_uniforms = camera.update_uniforms(&renderer);

        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs_f32();

        let uniforms = Uniforms {
            time,
            frame: *frame_count,
            _padding: [0, 0],
            camera_uniforms,
        };

        renderer
            .queue()
            .write_buffer(&self.uniforms, 0, bytemuck::cast_slice(&[uniforms]));

        *frame_count += 1;
    }

    pub fn compute(&self, renderer: &Renderer, compute_pass: &mut wgpu::ComputePass<'_>) {
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &self.bind_group, &[]);

        let surface_config = renderer.surface_config();
        let workgroup_x = (surface_config.width + 15) / 16;
        let workgroup_y = (surface_config.height + 15) / 16;
        compute_pass.dispatch_workgroups(workgroup_x, workgroup_y, 1);
    }
}

impl RenderPass {
    pub fn new(renderer: &Renderer, raytracing_texture: &Texture) -> Self {
        let device = renderer.device();
        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Render Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&raytracing_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&raytracing_texture.sampler),
                },
            ],
        });

        let pipeline = create_render_pipeline(
            device,
            "Render Pipeline",
            &shader,
            &[Some(&bind_group_layout)],
            &[Some(wgpu::ColorTargetState {
                format: renderer.surface_config().format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            None,
        );

        Self {
            bind_group,
            pipeline,
        }
    }

    pub fn render(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..3, 0..1);
    }
}
