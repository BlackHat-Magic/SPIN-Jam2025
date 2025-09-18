pub mod model;
pub mod sprite;

use crate::*;

use model::{Model, ModelHandle};

use crate::physics::{Camera, Transform};

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use winit::window::Window;

use wgpu::util::DeviceExt;

fn look_at_rh(eye: [f32; 3], center: [f32; 3], up: [f32; 3]) -> [[f32; 4]; 4] {
    let eye = glam::Vec3::from(eye);
    let center = glam::Vec3::from(center);
    let up = glam::Vec3::from(up);

    let f = (center - eye).normalize();
    let s = f.cross(up).normalize();
    let u = s.cross(f);

    [
        [s.x, u.x, -f.x, 0.0],
        [s.y, u.y, -f.y, 0.0],
        [s.z, u.z, -f.z, 0.0],
        [-s.dot(eye), -u.dot(eye), f.dot(eye), 1.0],
    ]
}

fn perspective_rh(fovy_radians: f32, aspect: f32, near: f32, far: f32) -> [[f32; 4]; 4] {
    let f = 1.0 / (fovy_radians / 2.0).tan();
    [
        [f / aspect, 0.0, 0.0, 0.0],
        [0.0, f, 0.0, 0.0],
        [0.0, 0.0, (far + near) / (near - far), -1.0],
        [0.0, 0.0, (2.0 * far * near) / (near - far), 0.0],
    ]
}

#[derive(Copy, Clone)]
pub enum Align {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

pub trait Displayable {
    fn get_texture_and_size(&self) -> (&wgpu::Texture, wgpu::Extent3d);
}

impl Displayable for Box<dyn Displayable> {
    fn get_texture_and_size(&self) -> (&wgpu::Texture, wgpu::Extent3d) {
        (**self).get_texture_and_size()
    }
}

pub struct Quad {
    pub texture: Rc<wgpu::Texture>,
    pub rect: (f32, f32, f32, f32), // x, y, width, height
    pub depth: f32,
}

#[derive(Resource)]
pub struct Gpu {
    pub window: Arc<Window>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub surface: wgpu::Surface<'static>,
    pub surface_format: wgpu::TextureFormat,

    pub depth_texture: Option<wgpu::Texture>,

    pub quads: Vec<Quad>,
}

#[derive(Resource)]
pub struct Shaders {
    pub shaders: HashMap<String, wgpu::ShaderModule>,
    pub model_pipeline: wgpu::RenderPipeline,
    pub model_bind_group_layout: wgpu::BindGroupLayout,
}

impl Shaders {
    pub fn load(gpu: &Gpu) -> Self {
        let shaders = crate::gather_dir("shaders", |path| {
            let file_extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

            let shader = match file_extension {
                #[cfg(debug_assertions)]
                "wgsl" => gpu
                    .device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: path.to_str(),
                        source: wgpu::ShaderSource::Wgsl(
                            std::fs::read_to_string(&path)
                                .expect("Failed to read shader file")
                                .into(),
                        ),
                    }),
                #[cfg(not(debug_assertions))]
                "spv" => {
                    let shader_data: Vec<u8> =
                        std::fs::read(&path).expect("Failed to read shader file");
                    let source = wgpu::util::make_spirv(&shader_data);

                    gpu.device
                        .create_shader_module(wgpu::ShaderModuleDescriptor {
                            label: path.to_str(),
                            source,
                        })
                }
                _ => {
                    println!(
                        "Warning: Unsupported shader file extension: .{} at {:?}",
                        file_extension, path
                    );
                    return None;
                }
            };

            Some(shader)
        })
        .unwrap();

        let (model_pipeline, model_bind_group_layout) = Self::create_model_pipeline(gpu, &shaders);

        Self {
            shaders,
            model_pipeline,
            model_bind_group_layout,
        }
    }

    fn create_model_pipeline(
        gpu: &Gpu,
        shaders: &HashMap<String, wgpu::ShaderModule>,
    ) -> (wgpu::RenderPipeline, wgpu::BindGroupLayout) {
        let vs_module = shaders.get("vs_main").expect("vs_main shader not found");
        let fs_module = shaders.get("fg_main").expect("fg_main shader not found");

        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Model Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Model Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Model Pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: vs_module,
                    entry_point: Some("main"),
                    buffers: &[Model::get_vertex_layout()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: fs_module,
                    entry_point: Some("main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: gpu.surface_format,
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
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth24Plus,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            });

        (pipeline, bind_group_layout)
    }
}

system!(
    fn init_shaders(
        gpu: res &Gpu,
        commands: commands
    ) {
        let shaders = Shaders::load(gpu.unwrap());
        commands.insert_resource(shaders);
    }
);

#[derive(Resource)]
pub struct Models {
    pub models: HashMap<String, Model>,
}

impl Models {
    pub fn load(gpu: &Gpu) -> Self {
        let models = crate::gather_dir("models", |path| Model::load(path, gpu)).unwrap();

        Self { models }
    }
}

system!(
    fn init_models(
        gpu: res &Gpu,
        commands: commands,
    ) {
        let models = Models::load(gpu.unwrap());
        commands.insert_resource(models);
    }
);

impl Gpu {
    pub async fn new(window: Arc<Window>) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .unwrap();

        let size = window.inner_size();

        let surface = instance.create_surface(window.clone()).unwrap();
        let cap = surface.get_capabilities(&adapter);
        let surface_format = cap.formats[0];

        let mut state = Self {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,

            depth_texture: None,

            quads: Vec::new(),
        };

        state.configure_surface();

        state
    }

    pub fn get_window(&self) -> &Window {
        &self.window
    }

    pub fn configure_surface(&mut self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            view_formats: vec![],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);

        let depth_size = wgpu::Extent3d {
            width: self.size.width,
            height: self.size.height,
            depth_or_array_layers: 1,
        };

        let depth_desc = wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: depth_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let depth_texture = self.device.create_texture(&depth_desc);
        self.depth_texture = Some(depth_texture);
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        self.size = new_size;

        self.configure_surface();
    }

    pub fn display(
        &mut self,
        item: &dyn Displayable,
        location: (f32, f32),
        scale: (f32, f32),
        depth: f32,
        align: Align,
    ) {
        let (texture, size) = item.get_texture_and_size();
        let size = (size.width as f32 * scale.0, size.height as f32 * scale.1);

        let (x, y) = match align {
            Align::TopLeft => (location.0, location.1),
            Align::TopCenter => (location.0 - size.0 / 2.0, location.1),
            Align::TopRight => (location.0 - size.0, location.1),
            Align::CenterLeft => (location.0, location.1 - size.1 / 2.0),
            Align::Center => (location.0 - size.0 / 2.0, location.1 - size.1 / 2.0),
            Align::CenterRight => (location.0 - size.0, location.1 - size.1 / 2.0),
            Align::BottomLeft => (location.0, location.1 - size.1),
            Align::BottomCenter => (location.0 - size.0 / 2.0, location.1 - size.1),
            Align::BottomRight => (location.0 - size.0, location.1 - size.1),
        };

        let rect = (x, y, size.0, size.1);

        let quad = Quad {
            texture: Rc::new(texture.clone()),
            rect,
            depth,
        };
        self.insert_quad(quad);
    }

    fn insert_quad(&mut self, quad: Quad) {
        let pos = self
            .quads
            .binary_search_by(|q| q.depth.partial_cmp(&quad.depth).unwrap());
        let pos = match pos {
            Ok(pos) => pos,
            Err(pos) => pos,
        };
        self.quads.insert(pos, quad);
    }
}

system!(
    fn render_system(
        gpu: res &mut Gpu,
        shaders: res &Shaders,
        models: res &Models,

        to_display: query (&Transform, &ModelHandle),
        camera: query (&Transform, &Camera),
    ) {
        let (Some(gpu), Some(shaders), Some(models)) = (gpu, shaders, models) else {
            return;
        };

        let surface_texture = gpu
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(gpu.surface_format),
                ..Default::default()
            });

        let mut encoder = gpu.device.create_command_encoder(&Default::default());

        if let Some((transform, camera)) = camera.next() {
            let depth_view_option = gpu.depth_texture.as_ref().map(|tex| {
                tex.create_view(&wgpu::TextureViewDescriptor::default())
            });

            let mut renderpass_desc = wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            };

            if let Some(depth_view) = depth_view_option.as_ref() {
                renderpass_desc.depth_stencil_attachment = Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                });
            }

            let projection_matrix = camera.projection_matrix();
            let projection_matrix = projection_matrix.to_cols_array_2d();

            let view_matrix = transform.to_view_matrix();
            let view_matrix = view_matrix.to_cols_array_2d();

            let mut renderpass = encoder.begin_render_pass(&renderpass_desc);

            for model in to_display {
                let (transform, model_handle) = model;

                let Some(model) = models.models.get(&model_handle.path) else {
                    eprintln!("Model not found: {}", model_handle.path);
                    continue;
                };

                let model_matrix = transform.to_matrix();
                let model_matrix = model_matrix.to_cols_array_2d();


                let uniforms_data = [
                    model_matrix,
                    view_matrix,
                    projection_matrix,
                ];

                let uniforms_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Uniforms Buffer"),
                    contents: bytemuck::cast_slice(&uniforms_data),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

                let light_data: [f32; 8] = [
                    2.0, 5.0, -2.0, 0.0, // position (vec3 + padding)
                    1.0, 1.0, 1.0, 0.0, // color (vec3 + padding)
                ];

                let light_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Light Buffer"),
                    contents: bytemuck::cast_slice(&light_data),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

                let camera_data: [f32; 4] = [0.0, 0.0, 5.0, 0.0]; // position + padding
                let camera_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Camera Buffer"),
                    contents: bytemuck::cast_slice(&camera_data),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

                let material_data: [f32; 8] = [
                    1.0, 1.0, 0.0, 0.0, // yellow
                    0.6,                // metallic
                    0.1,                // roughness
                    1.0,                // ao
                    0.0,                // padding
                ];

                let material_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Material Buffer"),
                    contents: bytemuck::cast_slice(&material_data),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

                let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Model Bind Group"),
                    layout: &shaders.model_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: uniforms_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: light_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: camera_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: material_buffer.as_entire_binding(),
                        },
                    ],
                });

                renderpass.set_pipeline(&shaders.model_pipeline);
                renderpass.set_bind_group(0, &bind_group, &[]);
                model.render(&mut renderpass);
            }
        }

        {
            let index_buffer = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Quad Index Buffer"),
                    contents: bytemuck::cast_slice(&[0u16, 1, 2, 2, 3, 0]),
                    usage: wgpu::BufferUsages::INDEX,
                });

            let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            gpu.quads.iter().for_each(|quad| {
                let _texture_view = quad.texture.create_view(&Default::default());
                let buffer = gpu
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Quad Vertex Buffer"),
                        contents: bytemuck::cast_slice(&[
                            quad.rect.0,
                            quad.rect.1,
                            0.0,
                            0.0,
                            0.0,
                            quad.rect.0 + quad.rect.2,
                            quad.rect.1,
                            0.0,
                            1.0,
                            0.0,
                            quad.rect.0 + quad.rect.2,
                            quad.rect.1 + quad.rect.3,
                            0.0,
                            1.0,
                            1.0,
                            quad.rect.0,
                            quad.rect.1 + quad.rect.3,
                            0.0,
                            0.0,
                            1.0,
                        ]),
                        usage: wgpu::BufferUsages::VERTEX,
                    });

                // TODO: handle pipelines properly

                renderpass.set_vertex_buffer(0, buffer.slice(..));
                renderpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                renderpass.draw_indexed(0..6, 0, 0..1);

            });
        }

        gpu.queue.submit([encoder.finish()]);
        gpu.window.pre_present_notify();
        surface_texture.present();

        gpu.quads.clear();
        gpu.window.request_redraw();
    }
);
