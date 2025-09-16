use std::collections::HashMap;
use winit::{event::WindowEvent, keyboard::{KeyCode, PhysicalKey}};

use crate::*;

#[derive(Resource)]
pub struct WindowEvents {
    pub events: Option<WindowEvent>,
}

impl WindowEvents {
    pub fn new(events: Option<WindowEvent>) -> Self {
        Self { events }
    }
}

system!(
    fn input_system(
        input: res &mut Input,
        gpu: res &mut Gpu,
        events: res &mut WindowEvents,
    ) {
        let Some(events) = events else {
            return;
        };
        let Some(input) = input else {
            return;
        };
        let Some(gpu) = gpu else {
            return;
        };

        input.update(gpu, events);
    }
);

#[derive(Resource)]
pub struct Input {
    keys: HashMap<KeyCode, bool>,
}

impl Input {
    pub fn new() -> Self {
        Self { keys: HashMap::new() }
    }

    pub fn update(&mut self, gpu: &mut Gpu, events: &mut WindowEvents) -> bool {
        let mut exit = false;

        if let Some(event) = events.events.take() {
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
