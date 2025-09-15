pub mod sprite;

use crate::*;

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use winit::window::Window;

use bytemuck;
use wgpu::util::DeviceExt;
use wgpu::{self, StoreOp};

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

    pub shaders: HashMap<String, wgpu::ShaderModule>,
    pub quads: Vec<Quad>,
}

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

        let shaders = crate::gather_dir("shaders", |path| {
            let file_extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

            let shader = match file_extension {
                #[cfg(debug_assertions)]
                "wgsl" => device.create_shader_module(wgpu::ShaderModuleDescriptor {
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

                    device.create_shader_module(wgpu::ShaderModuleDescriptor {
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
        }).unwrap();

        let state = Self {
            window,
            device,
            queue,
            size,
            surface,
            surface_format,

            shaders,
            quads: Vec::new(),
        };

        state.configure_surface();

        state
    }

    pub fn get_window(&self) -> &Window {
        &self.window
    }

    pub fn configure_surface(&self) {
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: self.surface_format,
            view_formats: vec![self.surface_format.add_srgb_suffix()],
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            width: self.size.width,
            height: self.size.height,
            desired_maximum_frame_latency: 2,
            present_mode: wgpu::PresentMode::AutoVsync,
        };
        self.surface.configure(&self.device, &surface_config);
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

static mut IHADG: i32 = 0;

system!(
    fn render_system(
        gpu: res &mut Gpu,
    ) {
        let Some(gpu) = gpu else {
            return;
        };

        let surface_texture = gpu
            .surface
            .get_current_texture()
            .expect("failed to acquire next swapchain texture");
        let texture_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor {
                format: Some(gpu.surface_format.add_srgb_suffix()),
                ..Default::default()
            });

        let mut encoder = gpu.device.create_command_encoder(&Default::default());

        let renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &texture_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(if unsafe { IHADG > 0 } {
                        wgpu::Color::GREEN
                    } else {
                        wgpu::Color::BLACK
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        unsafe {
            IHADG += 1;
            if IHADG > 100 {
                IHADG = -100;
            }
        }

        drop(renderpass);

        let index_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Quad Index Buffer"),
                contents: bytemuck::cast_slice(&[0u16, 1, 2, 2, 3, 0]),
                usage: wgpu::BufferUsages::INDEX,
            });

        gpu.quads.iter().for_each(|quad| {
            // Draw each quad
            let texture_view = quad.texture.create_view(&Default::default());
            let buffer = gpu
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Quad Vertex Buffer"),
                    contents: bytemuck::cast_slice(&[
                        // x, y, z, u, v
                        quad.rect.0,
                        quad.rect.1,
                        0.0,
                        0.0,
                        0.0, // Top-left
                        quad.rect.0 + quad.rect.2,
                        quad.rect.1,
                        0.0,
                        1.0,
                        0.0, // Top-right
                        quad.rect.0 + quad.rect.2,
                        quad.rect.1 + quad.rect.3,
                        0.0,
                        1.0,
                        1.0, // Bottom-right
                        quad.rect.0,
                        quad.rect.1 + quad.rect.3,
                        0.0,
                        0.0,
                        1.0, // Bottom-left
                    ]),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            let mut renderpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // TODO: handle pipelines properly

            renderpass.set_vertex_buffer(0, buffer.slice(..));
            renderpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            renderpass.draw_indexed(0..6, 0, 0..1);

            drop(renderpass);
        });

        gpu.queue.submit([encoder.finish()]);
        gpu.window.pre_present_notify();
        surface_texture.present();

        gpu.quads.clear();
    }
);
