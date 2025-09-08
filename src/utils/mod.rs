use std::io::Result;
use std::path::PathBuf;

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
    std::fs::read_to_string(path)
}

pub fn load_resource_bytes(relative_path: &str) -> Result<Vec<u8>> {
    let path = get_resource_path(relative_path);
    std::fs::read(path)
}

pub fn load_resource_json<T: serde::de::DeserializeOwned>(relative_path: &str) -> Result<T> {
    let json = load_resource_string(relative_path)?;
    let data = serde_json::from_str(&json)?;
    Ok(data)
}

pub fn save_resource_string(relative_path: &str, data: &str) -> Result<()> {
    let path = get_resource_path(relative_path);
    std::fs::write(path, data)
}

pub fn save_resource_bytes(relative_path: &str, data: &[u8]) -> Result<()> {
    let path = get_resource_path(relative_path);
    std::fs::write(path, data)
}

pub fn save_resource_json<T: serde::ser::Serialize>(relative_path: &str, data: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(data)?;
    save_resource_string(relative_path, &json)
}
