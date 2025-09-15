use std::{collections::HashMap, sync::mpsc::Receiver};
use winit::{event::WindowEvent, keyboard::{KeyCode, PhysicalKey}};

use crate::*;

system!(
    fn input_system(
        input: res &mut Input,
        gpu: res &mut Gpu,
        commands: commands
    ) {
        let Some(input) = input else {
            return;
        };
        let Some(gpu) = gpu else {
            return;
        };

        if input.update(gpu) {
            commands.exit();
        }
    }
);

#[derive(Resource)]
pub struct Input {
    rx: Receiver<WindowEvent>,
    keys: HashMap<KeyCode, bool>,
}

impl Input {
    pub fn new(rx: Receiver<WindowEvent>) -> Self {
        Self { rx, keys: HashMap::new() }
    }

    pub fn update(&mut self, gpu: &mut Gpu) -> bool {
        let mut exit = false;

        while let Ok(event) = self.rx.try_recv() {
            match event {
                WindowEvent::KeyboardInput { event, .. } => {
                    match event.physical_key {
                        PhysicalKey::Code(keycode) => {
                            let pressed = event.state == winit::event::ElementState::Pressed;
                            _ = self.keys.insert(keycode, pressed);
                        }
                        _ => {}
                    }
                }
                WindowEvent::Resized(physical_size) => {
                    gpu.resize(physical_size);
                }
                WindowEvent::CloseRequested => {
                    exit = true;
                }
                _ => {}
            }
        }
        exit
    }
}
