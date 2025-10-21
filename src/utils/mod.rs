use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::utils::input::Input;
use crate::*;

pub mod input;
pub mod time;

pub struct UtilPlugin {
    is_server: bool,
}

impl UtilPlugin {
    pub fn client() -> Self {
        Self { is_server: false }
    }

    pub fn server() -> Self {
        Self { is_server: true }
    }
}
impl Plugin for UtilPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Input::new());

        if !self.is_server {
            app.add_system(input::input_system, SystemStage::PreUpdate);
        }
        app.add_system(time::update_time, SystemStage::PreUpdate);
        app.add_system(time::init_time, SystemStage::Init);
    }
}

pub fn get_resource_path(relative_path: &str) -> PathBuf {
    let path = std::env::current_exe().expect("Can't find path to executable");
    let path = format!(
        "{}/resources/{}",
        path.parent().unwrap().display(),
        relative_path
    );

    PathBuf::from(path)
}

pub fn load_resource_string(relative_path: &str) -> Result<String> {
    let path = get_resource_path(relative_path);
    let content = std::fs::read_to_string(path)?;
    Ok(content)
}

pub fn load_resource_bytes(relative_path: &str) -> Result<Vec<u8>> {
    let path = get_resource_path(relative_path);
    let bytes = std::fs::read(path)?;
    Ok(bytes)
}

pub fn load_resource_json<T: serde::de::DeserializeOwned>(relative_path: &str) -> Result<T> {
    let json = load_resource_string(relative_path)?;
    let data = serde_json::from_str(&json)?;
    Ok(data)
}

pub fn save_resource_string(relative_path: &str, data: &str) -> Result<()> {
    let path = get_resource_path(relative_path);
    std::fs::write(path, data)?;
    Ok(())
}

pub fn save_resource_bytes(relative_path: &str, data: &[u8]) -> Result<()> {
    let path = get_resource_path(relative_path);
    std::fs::write(path, data)?;
    Ok(())
}

pub fn save_resource_json<T: serde::ser::Serialize>(relative_path: &str, data: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(data)?;
    save_resource_string(relative_path, &json)
}

fn gather_all_files(root: &PathBuf) -> Result<Vec<PathBuf>> {
    let read_dir = std::fs::read_dir(root)?;
    let mut files = Vec::new();

    for entry in read_dir {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(gather_all_files(&path)?);
        } else {
            files.push(path);
        }
    }

    Ok(files)
}

pub fn gather_dir<T>(
    dir: &str,
    mut filter_map: impl FnMut(&PathBuf) -> Option<T>,
) -> Result<HashMap<String, T>> {
    let mut results = HashMap::new();
    let path = get_resource_path(dir);
    for file in gather_all_files(&path)? {
        if let Some(result) = filter_map(&file) {
            let file_extension = file.extension().and_then(|s| s.to_str()).unwrap_or("");

            let relative_dir = file
                .strip_prefix(&path)
                .unwrap()
                .to_str()
                .unwrap()
                .strip_suffix(&format!(".{}", file_extension))
                .unwrap()
                .to_string();

            #[cfg(target_os = "windows")]
            let relative_dir = relative_dir.replace("\\", "/");

            results.insert(relative_dir, result);
        }
    }
    Ok(results)
}
