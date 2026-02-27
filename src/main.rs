use std::sync::Arc;

use egui_wgpu::RendererOptions;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Fullscreen, Window, WindowId};

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
    window: Option<Arc<Window>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    surface: Option<wgpu::Surface<'static>>,
    surface_config: Option<wgpu::SurfaceConfiguration>,

    compute_pipeline: Option<wgpu::ComputePipeline>,
    compute_bind_group: Option<wgpu::BindGroup>,
    render_pipeline: Option<wgpu::RenderPipeline>,
    render_bind_group: Option<wgpu::BindGroup>,

    egui_state: Option<egui_winit::State>,
    egui_renderer: Option<egui_wgpu::Renderer>,

    last_frame_time: Option<std::time::Instant>,
    fps: f32,
}

impl App {
    fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))?;

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                ..Default::default()
            }))?;

        let surface = instance.create_surface(self.window.as_ref().unwrap().clone())?;

        let mut surface_config = surface
            .get_default_config(
                &adapter,
                self.window.as_ref().unwrap().inner_size().width,
                self.window.as_ref().unwrap().inner_size().height,
            )
            .unwrap();
        surface_config.present_mode = wgpu::PresentMode::Fifo;
        log::info!("Surface config: {:#?}", surface_config);
        surface.configure(&device, &surface_config);

        let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

        // Texture for compute shader
        let texture_size = wgpu::Extent3d {
            width: surface_config.width,
            height: surface_config.height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Compute Texture"),
            size: texture_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        // Compute pipeline
        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                }],
            });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute Pipeline Layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute Pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &shader,
            entry_point: Some("cs_main"),
            compilation_options: Default::default(),
            cache: None,
        });

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Compute Bind Group"),
            layout: &compute_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            }],
        });

        // Render pipeline
        let render_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[&render_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Render Bind Group"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        self.device = Some(device);
        self.queue = Some(queue);
        self.surface = Some(surface);

        let context = egui::Context::default();
        let viewport_id = context.viewport_id();
        let egui_state = egui_winit::State::new(
            context,
            viewport_id,
            self.window.as_ref().unwrap(),
            Some(self.window.as_ref().unwrap().scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            self.device.as_ref().unwrap(),
            surface_config.format,
            RendererOptions {
                ..Default::default()
            },
        );

        self.egui_state = Some(egui_state);
        self.egui_renderer = Some(egui_renderer);

        self.surface_config = Some(surface_config);

        self.compute_pipeline = Some(compute_pipeline);
        self.compute_bind_group = Some(compute_bind_group);
        self.render_pipeline = Some(render_pipeline);
        self.render_bind_group = Some(render_bind_group);

        self.last_frame_time = Some(std::time::Instant::now());

        Ok(())
    }

    fn render(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let output = match self.surface.as_ref().unwrap().get_current_texture() {
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

        // FPS Calculation
        if let Some(last_time) = self.last_frame_time {
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(last_time).as_millis() as f32;
            self.fps = 1000.0 / elapsed;
            self.last_frame_time = Some(now);
        }

        // Egui Frame Update
        let raw_input = self
            .egui_state
            .as_mut()
            .unwrap()
            .take_egui_input(self.window.as_ref().unwrap());

        let full_output = self
            .egui_state
            .as_mut()
            .unwrap()
            .egui_ctx()
            .run(raw_input, |ctx| {
                egui::Window::new("Stats").show(ctx, |ui| {
                    ui.label(format!("FPS: {:.1}", self.fps));
                });
            });

        let clipped_primitives = self
            .egui_state
            .as_mut()
            .unwrap()
            .egui_ctx()
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        // We only use one encoder for everything to avoid lifetime complexites
        let mut encoder =
            self.device
                .as_ref()
                .unwrap()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        // Update Egui buffers
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [
                self.surface_config.as_ref().unwrap().width,
                self.surface_config.as_ref().unwrap().height,
            ],
            pixels_per_point: self.window.as_ref().unwrap().scale_factor() as f32,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.as_mut().unwrap().update_texture(
                self.device.as_ref().unwrap(),
                self.queue.as_ref().unwrap(),
                *id,
                image_delta,
            );
        }

        self.egui_renderer.as_mut().unwrap().update_buffers(
            self.device.as_ref().unwrap(),
            self.queue.as_ref().unwrap(),
            &mut encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(self.compute_pipeline.as_ref().unwrap());
            compute_pass.set_bind_group(0, self.compute_bind_group.as_ref().unwrap(), &[]);

            let surface_config = self.surface_config.as_ref().unwrap();
            let workgroup_x = (surface_config.width + 15) / 16;
            let workgroup_y = (surface_config.height + 15) / 16;
            compute_pass.dispatch_workgroups(workgroup_x, workgroup_y, 1);
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

            render_pass.set_pipeline(self.render_pipeline.as_ref().unwrap());
            render_pass.set_bind_group(0, self.render_bind_group.as_ref().unwrap(), &[]);
            render_pass.draw(0..3, 0..1);

            // Draw egui
            self.egui_renderer.as_mut().unwrap().render(
                &mut render_pass.forget_lifetime(),
                &clipped_primitives,
                &screen_descriptor,
            );
        }

        self.queue.as_ref().unwrap().submit(Some(encoder.finish()));
        output.present();

        // Cleanup egui textures
        for x in &full_output.textures_delta.free {
            self.egui_renderer.as_mut().unwrap().free_texture(x);
        }

        Ok(())
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.window = Some(Arc::new(
                event_loop
                    .create_window(
                        Window::default_attributes()
                            .with_title("Ray Tracing")
                            .with_fullscreen(Some(Fullscreen::Borderless(None))),
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
        if let Some(egui_state) = self.egui_state.as_mut() {
            let response = egui_state.on_window_event(self.window.as_ref().unwrap(), &event);
            if response.repaint {
                self.window.as_ref().unwrap().request_redraw();
            }
            if response.consumed {
                return;
            }
        }

        match event {
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
