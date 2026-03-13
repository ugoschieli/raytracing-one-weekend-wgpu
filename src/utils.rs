use std::sync::Arc;

use winit::{dpi::PhysicalSize, window::Window};

pub(crate) fn create_instance(desc: Option<wgpu::InstanceDescriptor>) -> wgpu::Instance {
    let descriptor = desc.unwrap_or(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let instance = wgpu::Instance::new(&descriptor);

    instance
}

pub(crate) fn create_adapter(
    instance: &wgpu::Instance,
    desc: Option<wgpu::RequestAdapterOptions>,
) -> wgpu::Adapter {
    let descriptor = desc.unwrap_or(wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    });

    let adapter = pollster::block_on(instance.request_adapter(&descriptor)).unwrap();

    adapter
}

pub(crate) fn create_device(
    adapter: &wgpu::Adapter,
    desc: Option<wgpu::DeviceDescriptor>,
) -> (wgpu::Device, wgpu::Queue) {
    let descriptor = desc.unwrap_or(wgpu::DeviceDescriptor {
        ..Default::default()
    });

    let (device, queue) = pollster::block_on(adapter.request_device(&descriptor)).unwrap();

    (device, queue)
}

pub(crate) fn create_surface<'window>(
    instance: &wgpu::Instance,
    window: Arc<Window>,
) -> wgpu::Surface<'window> {
    let surface = instance.create_surface(window).unwrap();

    surface
}

pub(crate) fn configure_surface(
    surface: &wgpu::Surface,
    device: &wgpu::Device,
    adapter: &wgpu::Adapter,
    size: PhysicalSize<u32>,
    config: Option<wgpu::SurfaceConfiguration>,
) -> wgpu::SurfaceConfiguration {
    let mut config = config.unwrap_or(
        surface
            .get_default_config(adapter, size.width, size.height)
            .expect("Failed to get default surface configuration"),
    );
    config.format = wgpu::TextureFormat::Bgra8Unorm;

    surface.configure(device, &config);

    config
}

pub fn create_compute_pipeline(
    device: &wgpu::Device,
    label: &str,
    shader: &wgpu::ShaderModule,
    bind_groups: &[&wgpu::BindGroupLayout],
) -> wgpu::ComputePipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(format!("{} layout", label).as_str()),
        bind_group_layouts: bind_groups,
        push_constant_ranges: &[],
    });

    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        module: shader,
        entry_point: None,
        layout: Some(&layout),
        cache: None,
        compilation_options: wgpu::PipelineCompilationOptions::default(),
    })
}

pub fn create_render_pipeline(
    device: &wgpu::Device,
    label: &str,
    shader: &wgpu::ShaderModule,
    bind_groups: &[&wgpu::BindGroupLayout],
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(format!("{} layout", label).as_str()),
        bind_group_layouts: bind_groups,
        push_constant_ranges: &[],
    });

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Bgra8Unorm,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}
