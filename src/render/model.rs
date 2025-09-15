use std::path::PathBuf;

use crate::*;

pub struct Model {

}

impl Model {
    fn load_obj(path: &PathBuf, gpu: &Gpu) -> Option<Self> {
        todo!()
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
}