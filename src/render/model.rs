use std::path::PathBuf;

use wgpu::util::DeviceExt;
use crate::*;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

pub struct Model {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub vertex_count: u32,
    pub index_count: u32,
}

impl Model {
    pub fn load_obj(path: &PathBuf, gpu: &Gpu) -> Option<Self> {
        println!("Loading OBJ file: {:?}", path);
        let contents = std::fs::read_to_string(path).ok()?;
        println!("OBJ file contents length: {}", contents.len());

        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for line in contents.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            match parts[0] {
                "v" => {
                    if parts.len() >= 4 {
                        positions.push([
                            parts[1].parse().ok()?,
                            parts[2].parse().ok()?,
                            parts[3].parse().ok()?,
                        ]);
                    }
                }
                "vn" => {
                    if parts.len() >= 4 {
                        normals.push([
                            parts[1].parse().ok()?,
                            parts[2].parse().ok()?,
                            parts[3].parse().ok()?,
                        ]);
                    }
                }
                "vt" => {
                    if parts.len() >= 3 {
                        uvs.push([
                            parts[1].parse().ok()?,
                            parts[2].parse().ok()?,
                        ]);
                    }
                }
                "f" => {
                    if parts.len() >= 4 {
                        let mut face_indices = Vec::new();
                        for i in 1..parts.len() {
                            let indices_str: Vec<&str> = parts[i].split('/').collect();
                            if indices_str.len() >= 3 {
                                let pos_idx: usize = indices_str[0].parse().ok()?;
                                let uv_idx: usize = indices_str[1].parse().ok()?;
                                let normal_idx: usize = indices_str[2].parse().ok()?;

                                if pos_idx > 0 && uv_idx > 0 && normal_idx > 0 &&
                                   pos_idx <= positions.len() && uv_idx <= uvs.len() && normal_idx <= normals.len() {
                                    face_indices.push((pos_idx - 1, uv_idx - 1, normal_idx - 1));
                                }
                            }
                        }

                        if face_indices.len() >= 3 {
                            for i in 1..face_indices.len() - 1 {
                                let v0 = face_indices[0];
                                let v1 = face_indices[i];
                                let v2 = face_indices[i + 1];

                                // For simplicity, just add new vertices for each triangle
                                // In a real implementation, you'd want to deduplicate vertices
                                for &(pos, uv, normal) in &[v0, v1, v2] {
                                    vertices.push(Vertex {
                                        position: positions[pos],
                                        normal: normals[normal],
                                        uv: uvs[uv],
                                    });
                                }

                                let base_index = vertices.len() as u16 - 3;
                                indices.extend_from_slice(&[base_index, base_index + 1, base_index + 2]);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        println!("Loaded {} vertices, {} indices", vertices.len(), indices.len());

        if vertices.is_empty() {
            println!("No vertices loaded from OBJ file");
            return None;
        }

        let vertex_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("OBJ Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("OBJ Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Some(Model {
            vertex_buffer,
            index_buffer,
            vertex_count: vertices.len() as u32,
            index_count: indices.len() as u32,
        })
    }

    pub fn get_vertex_layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3, // position
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3, // normal
                },
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() + std::mem::size_of::<[f32; 3]>()) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2, // uv
                },
            ],
        }
    }

    pub fn load(path: &PathBuf, gpu: &Gpu) -> Option<Self> {
        let file_extension = path.extension()?.to_str()?;
        match file_extension {
            "obj" => {
                Self::load_obj(path, gpu)
            },
            _ => {
                eprintln!("Unsupported model format: {}", file_extension);
                None
            }
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}