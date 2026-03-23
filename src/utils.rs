use std::sync::Arc;

use winit::{dpi::PhysicalSize, window::Window};

pub fn create_wgpu_instance() -> wgpu::Instance {
    wgpu::Instance::default()
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
    bind_groups: &[Option<&wgpu::BindGroupLayout>],
) -> wgpu::ComputePipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(format!("{} layout", label).as_str()),
        bind_group_layouts: bind_groups,
        immediate_size: 0,
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
    bind_groups: &[Option<&wgpu::BindGroupLayout>],
    color_targets: &[Option<wgpu::ColorTargetState>],
    depth_stencil: Option<wgpu::DepthStencilState>,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(format!("{} layout", label).as_str()),
        bind_group_layouts: bind_groups,
        immediate_size: 0,
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
            targets: color_targets,
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
    })
}
