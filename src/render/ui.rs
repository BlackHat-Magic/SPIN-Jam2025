use crate::render::sprite::*;
use crate::utils::*;
use crate::*;

use fontdb::{self, ID};
use glyphon::{Cache, Color, FontSystem, Resolution, TextAtlas, TextBounds, TextRenderer};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        let gpu = app.get_resource::<Gpu>().unwrap();
        let images = app.get_resource::<Images>().unwrap();

        let (ui_state, ui_nodes) = UiState::load(gpu, images);
        app.insert_resource(ui_state);
        app.insert_resource(ui_nodes);
        app.add_system(display_ui, SystemStage::PostUpdate);
    }
}

system! {
    fn display_ui(
        ui: res &mut UiState,
        ui_nodes: res &mut UiNodes,
        gpu: res &mut Gpu,
    ) {
        let (Some(ui), Some(ui_nodes), Some(gpu)) = (ui, ui_nodes, gpu) else {
            return;
        };

        ui_nodes.root.display(ui, gpu);
    }
}

#[derive(Resource)]
pub struct UiState {
    toggles: HashSet<String>,
}

#[derive(Resource)]
pub struct UiNodes {
    root: UiNode,
}

impl UiState {
    fn load(gpu: &Gpu, images: &Images) -> (Self, UiNodes) {
        let mut font_db = fontdb::Database::new();
        let font_map = gather_dir("fonts", |path| {
            if !path.extension()
                .and_then(|s| s.to_str())
                .map(|s| matches!(s, "ttf" | "otf" | "woff" | "woff2"))
                .unwrap_or(false) {
                    return None;
                }

            Some(font_db.load_font_source(fontdb::Source::File(path.to_path_buf()))[0])
        }).expect("could not load fonts");
        let mut fonts = FontSystem::new_with_locale_and_db("US".to_string(), font_db);

        let nodes = gather_dir("ui", |path| {
            let file = std::fs::read_to_string(path).ok()?;
            serde_json::from_str::<SerializedUiNode>(&file).ok()
        })
        .unwrap();
        let root = UiNode::from_serialized(nodes.get("root").unwrap(), &nodes, gpu, &mut fonts, images, &font_map);

        (
            UiState {
                toggles: HashSet::new(),
            },
            UiNodes { root },
        )
    }
}

#[derive(Deserialize, Copy, Clone)]
struct Rect {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum SerializedUiNode {
    Container {
        toggle_id: Option<String>,
        rect: Rect,
        id: String,
        children: Vec<SerializedUiNode>,
    },
    Text {
        rect: Rect,
        id: String,
        content: String,
        font: String,
        size: f32,
    },
    Image {
        rect: Rect,
        id: String,
        image: String,
    },
    SubFile {
        file_path: String,
    },
}

enum UiNode {
    Container {
        toggle_id: Option<String>,
        rect: Rect,
        id: String,
        children: Vec<UiNode>,
    },
    Text {
        rect: Rect,
        text_displayable: TextDisplayable,
        id: String,
    },
    Image {
        rect: Rect,
        id: String,
        image: super::sprite::Sprite,
    },
}

impl UiNode {
    fn from_serialized(
        node: &SerializedUiNode,
        nodes: &HashMap<String, SerializedUiNode>,
        gpu: &Gpu,
        fonts: &mut FontSystem,
        images: &Images,
        font_map: &HashMap<String, ID>,
    ) -> Self {
        match node {
            SerializedUiNode::Container {
                toggle_id,
                rect,
                id,
                children,
            } => Self::Container {
                toggle_id: toggle_id.clone(),
                rect: *rect,
                id: id.clone(),
                children: children
                    .iter()
                    .map(|node| UiNode::from_serialized(node, nodes, gpu, fonts, images, font_map))
                    .collect(),
            },
            SerializedUiNode::Text {
                rect,
                id,
                content,
                font,
                size,
            } => {
                let mut text_displayable =
                    TextDisplayable::new(content.clone(), *font_map.get(font).unwrap(), *size);
                text_displayable.prepare(&gpu, fonts).expect(&format!("failed to prepare text {}", &content));
                Self::Text {
                    rect: *rect,
                    text_displayable,
                    id: id.clone(),
                }
        },
            SerializedUiNode::Image { rect, id, image } => Self::Image {
                rect: *rect,
                id: id.clone(),
                image: {
                    SpriteBuilder {
                        image_path: image.clone(),
                        ..Default::default()
                    }
                    .build(gpu, images)
                },
            },
            SerializedUiNode::SubFile { file_path } => {
                UiNode::from_serialized(nodes.get(file_path).unwrap(), nodes, gpu, fonts, images, font_map)
            }
        }
    }

    fn display(&self, ui: &mut UiState, gpu: &mut Gpu) {
        match self {
            UiNode::Container {
                toggle_id,
                rect: _,
                id: _,
                children,
            } => {
                let should_display = if let Some(toggle_id) = toggle_id {
                    !ui.toggles.contains(toggle_id)
                } else {
                    true
                };
                if should_display {
                    for child in children {
                        child.display(ui, gpu);
                    }
                }
            }
            UiNode::Text {
                rect,
                id: _,
                text_displayable,
            } => {
                gpu.display(
                    text_displayable,
                    (rect.x, rect.y),
                    (rect.width, rect.height),
                    0.0,
                    Align::TopLeft,
                );
            }
            UiNode::Image { rect, id, image } => {
                gpu.display(
                    image,
                    (rect.x, rect.y),
                    (rect.width, rect.height),
                    0.0,
                    Align::TopLeft,
                );
            }
        }
    }
}

pub struct TextDisplayable {
    content: String,
    font: ID,
    size: f32,
    texture: Option<wgpu::Texture>,
    extent: Option<wgpu::Extent3d>,
}

impl TextDisplayable {
    pub fn new(content: String, font: ID, size: f32) -> Self {
        Self {
            content,
            font,
            size,
            texture: None,
            extent: None,
        }
    }

    pub fn prepare(&mut self, gpu: &Gpu, fonts: &mut FontSystem) -> anyhow::Result<()> {
        let cache = Cache::new(&gpu.device);
        let mut atlas = TextAtlas::new(
            &gpu.device,
            &gpu.queue,
            &cache,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        );
        let mut swash_cache = glyphon::SwashCache::new();

        let mut renderer = TextRenderer::new(
            &mut atlas,
            &gpu.device,
            wgpu::MultisampleState::default(),
            None,
        );

        let metrics = glyphon::Metrics::new(self.size, self.size * 1.2); // scale and line_height
        let mut buffer = glyphon::Buffer::new(fonts, metrics);
        let attrs = glyphon::Attrs::new();
        buffer.set_text(fonts, &self.content, &attrs, glyphon::Shaping::Advanced);
        buffer.shape_until_scroll(fonts, false);

        let width = buffer
            .layout_runs()
            .map(|run| run.line_w as u32)
            .max()
            .unwrap_or(1);
        let height = buffer.layout_runs().count() as u32 * (self.size as u32);

        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Text Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Text Render Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Text Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &texture_view, // Write to the texture
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            let mut viewport = glyphon::Viewport::new(&gpu.device, &cache);

            viewport.update(&gpu.queue, Resolution {
                width,
                height,
            });

            let text_areas = vec![glyphon::TextArea {
                buffer: &buffer,
                left: 0.0,
                top: 0.0,
                scale: 1.0,
                bounds: TextBounds::default(),
                default_color: Color::rgb(255, 255, 255),
                custom_glyphs: &[],
            }];

            renderer.prepare(&gpu.device, &gpu.queue, fonts, &mut atlas, &viewport, text_areas, &mut swash_cache)?;
            renderer.render(&atlas, &viewport, &mut render_pass).unwrap();

            atlas.trim();
        }
        gpu.queue.submit(Some(encoder.finish()));

        self.texture = Some(texture);
        self.extent = Some(wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        });

        Ok(())
    }
}

impl Displayable for TextDisplayable {
    fn get_texture_and_size(&self) -> (&wgpu::Texture, wgpu::Extent3d) {
        (
            self.texture.as_ref().expect("Texture not prepared"),
            self.extent.expect("Extent not prepared"),
        )
    }
}
