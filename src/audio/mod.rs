use crate::*;

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

use rodio::source::*;
use rodio::*;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Audio::load());
    }
}

#[derive(Resource)]
pub struct Audio {
    stream_handle: OutputStream,
    sounds: HashMap<String, Buffered<Decoder<BufReader<File>>>>,
}

impl Audio {
    fn load() -> Self {
        let stream_handle = OutputStreamBuilder::open_default_stream().unwrap();
        let sounds = gather_dir("sounds", |path| {
            let file = File::open(path).ok()?;
            let buf_reader = BufReader::new(file);
            Some(Decoder::new(buf_reader).ok()?.buffered())
        })
        .unwrap();

        Self {
            stream_handle,
            sounds,
        }
    }

    pub fn play(&self, name: &str, volume: f32, looping: bool) {
        if let Some(sound) = self.sounds.get(name) {
            let sink = Sink::connect_new(self.stream_handle.mixer());
            let sound = (*sound).clone();
            //let sound = sound.amplify(volume);
            if looping {
                let sound = sound.repeat_infinite();
                sink.append(sound);
            } else {
                sink.append(sound);
            }

            sink.detach();
        } else {
            println!("Sound '{}' not found!", name);
        }
    }
}
