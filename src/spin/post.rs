use crate::*;
use glam::Vec2;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct PostParams {
    strength: f32,
    time: f32,
    aspect: f32,
    _pad: f32,
}

pub struct PostProcessPlugin;

impl Plugin for PostProcessPlugin {
    fn build(&self, app: &mut App) {
        // Build once we have a Gpu
        let gpu = app.get_resource::<render::Gpu>().expect("Gpu missing for PostProcess");

        // Load shader
        let wgsl = utils::load_resource_string("shaders/spin_post.wgsl").expect("spin_post.wgsl missing");
        let module = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Spin PostProcess Shader"),
            source: wgpu::ShaderSource::Wgsl(wgsl.into()),
        });

        // Layout: texture + sampler + uniforms
        let bgl = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Spin PostProcess BGL"),
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
            ],
        });

        let pl_layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Spin PostProcess PipelineLayout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Spin PostProcess Pipeline"),
            layout: Some(&pl_layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                buffers: &[], // full-screen triangle, no vbuf
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor::default());

        // Initial params
        let aspect = gpu.size.width.max(1) as f32 / gpu.size.height.max(1) as f32;
        let params = PostParams { strength: 1.0, time: 0.0, aspect, _pad: 0.0 };
        let params_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Spin PostProcess Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Insert render::PostProcessState so the renderer will use it
        app.insert_resource(render::PostProcessState {
            pipeline,
            bind_group_layout: bgl,
            sampler,
            params_buffer,
            intermediate: None,
            last_size: (0, 0),
        });

        // Update params every frame
        app.add_system(update_post_params, SystemStage::PreUpdate);
    }
}

system! {
    fn update_post_params(
        gpu: res &Gpu,
        time: res &Time,
        post: res &mut render::PostProcessState,
    ) {
        let (Some(gpu), Some(time), Some(post)) = (gpu, time, post) else { return; };

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct PostParams { strength: f32, time: f32, aspect: f32, _pad: f32 }

        let aspect = (gpu.size.width.max(1) as f32) / (gpu.size.height.max(1) as f32);
        let params = PostParams { strength: 1.0, time: time.delta_seconds + 0.0, aspect, _pad: 0.0 };

        gpu.queue.write_buffer(&post.params_buffer, 0, bytemuck::bytes_of(&params));
    }
}