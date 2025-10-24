use crate::render::sprite::*;
use crate::utils::*;
use crate::*;

use glyphon::*;
use serde::Deserialize;
use std::collections::HashMap;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        let gpu = app.get_resource::<Gpu>().unwrap();
        let images = app.get_resource::<Images>().unwrap();

        app.insert_resource(Ui::load(gpu, images));
        app.add_system(display_ui, SystemStage::PostUpdate);
    }
}

system! {
    fn display_ui(
        ui: res &Ui,
        gpu: res &mut Gpu,
    ) {
        let (Some(ui), Some(gpu)) = (ui, gpu) else {
            return;
        };

        ui.display(gpu);
    }
}

#[derive(Resource)]
pub struct Ui {
    root: UiNode,
    fonts: FontSystem,
    ui_toggles: HashMap<String, bool>,
}

impl Ui {
    fn load(gpu: &Gpu, images: &Images) -> Self {
        let fonts = get_resource_path("fonts");
        let fonts = gather_all_files(&fonts).expect("could not locate fonts folder");
        let fonts = fonts.into_iter().filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .map(|s| matches!(s, "ttf" | "otf" | "woff" | "woff2"))
                .unwrap_or(false)
        });
        let fonts = FontSystem::new_with_fonts(fonts.map(|p| fontdb::Source::File(p)));

        let nodes = gather_dir("ui", |path| {
            let file = std::fs::read_to_string(path).ok()?;
            serde_json::from_str::<SerializedUiNode>(&file).ok()
        })
        .unwrap();
        let root = UiNode::from_serialized(nodes.get("root").unwrap(), &nodes, gpu, images);

        Ui {
            root,
            fonts,
            ui_toggles: HashMap::new(),
        }
    }

    fn display(&self, gpu: &mut Gpu) {}
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
    Button {
        rect: Rect,
        id: String,
        label: String,
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
        id: String,
        content: String,
        font: String,
        size: f32,
    },
    Button {
        rect: Rect,
        id: String,
        label: String,
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
        images: &Images,
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
                    .map(|node| UiNode::from_serialized(node, nodes, gpu, images))
                    .collect(),
            },
            SerializedUiNode::Text {
                rect,
                id,
                content,
                font,
                size,
            } => Self::Text {
                rect: *rect,
                id: id.clone(),
                content: content.clone(),
                font: font.clone(),
                size: *size,
            },
            SerializedUiNode::Button { rect, id, label } => Self::Button {
                rect: *rect,
                id: id.clone(),
                label: label.clone(),
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
                UiNode::from_serialized(nodes.get(file_path).unwrap(), nodes, gpu, images)
            }
        }
    }
}
