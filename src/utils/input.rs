use std::collections::HashMap;
use winit::{
    event::WindowEvent,
    keyboard::{KeyCode, PhysicalKey},
};

use crate::*;

#[derive(Resource)]
pub struct WindowEvents {
    pub events: Vec<WindowEvent>,
}

impl WindowEvents {
    pub fn new(events: Vec<WindowEvent>) -> Self {
        Self { events }
    }
}

#[derive(Resource)]
pub struct DeviceEvents {
    pub events: Vec<winit::event::DeviceEvent>,
}

impl DeviceEvents {
    pub fn new(events: Vec<winit::event::DeviceEvent>) -> Self {
        Self { events }
    }
}

system!(
    fn input_system(
        input: res &mut Input,
        gpu: res &mut Gpu,
        events: res &mut WindowEvents,
        device_events: res &mut DeviceEvents,
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
        let Some(device_events) = device_events else {
            return;
        };

        input.update(gpu, events, device_events);
    }
);

#[derive(Resource)]
pub struct Input {
    keys: HashMap<KeyCode, bool>,
    key_just_pressed: HashMap<KeyCode, bool>,
    mouse_buttons: HashMap<winit::event::MouseButton, bool>,
    mouse_buttons_just_pressed: HashMap<winit::event::MouseButton, bool>,
    prev_mouse_pos: (f64, f64),
    mouse_delta: (f64, f64),
    cursor_in_window: bool,
    pub cursor_grabbed: bool,
}

impl Input {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
            key_just_pressed: HashMap::new(),
            mouse_buttons: HashMap::new(),
            mouse_buttons_just_pressed: HashMap::new(),
            prev_mouse_pos: (0.0, 0.0),
            mouse_delta: (0.0, 0.0),
            cursor_in_window: false,
            cursor_grabbed: false,
        }
    }

    pub fn update(&mut self, gpu: &mut Gpu, events: &mut WindowEvents, device_events: &mut DeviceEvents) {
        self.key_just_pressed.clear();
        self.mouse_buttons_just_pressed.clear();
        let mut mouse_delta = (0.0, 0.0);

        for event in events.events.drain(..) {
            match event {
                WindowEvent::KeyboardInput { event, .. } => match event.physical_key {
                    PhysicalKey::Code(keycode) => {
                        let pressed = event.state == winit::event::ElementState::Pressed;
                        _ = self.keys.insert(keycode, pressed);
                        _ = self.key_just_pressed.insert(keycode, pressed);
                    }
                    _ => {}
                },
                WindowEvent::MouseInput { state, button, .. } => {
                    let pressed = state == winit::event::ElementState::Pressed;
                    _ = self.mouse_buttons.insert(button, pressed);
                    _ = self.mouse_buttons_just_pressed.insert(button, pressed);
                }
                WindowEvent::Resized(physical_size) => {
                    gpu.resize(physical_size);
                }
                WindowEvent::CursorMoved { position, .. } => {
                    // Only use this for delta if NOT grabbed (raw motion takes priority when grabbed)
                    if !self.cursor_grabbed {
                        if !self.cursor_in_window {
                            self.prev_mouse_pos = (position.x, position.y);
                            self.cursor_in_window = true;
                        }

                        mouse_delta.0 = position.x - self.prev_mouse_pos.0;
                        mouse_delta.1 = position.y - self.prev_mouse_pos.1;
                        self.prev_mouse_pos = (position.x, position.y);
                    }
                }
                WindowEvent::CursorEntered { .. } => {
                    self.cursor_in_window = false;
                }
                _ => {}
            }
        }

        // Process device events for raw mouse motion (when grabbed)
        for event in device_events.events.drain(..) {
            match event {
                winit::event::DeviceEvent::MouseMotion { delta } => {
                    // Use raw delta when cursor is grabbed
                    if self.cursor_grabbed {
                        mouse_delta.0 += delta.0;
                        mouse_delta.1 += delta.1;
                    }
                }
                _ => {}
            }
        }

        if self.cursor_grabbed {
            let _ = gpu.window.set_cursor_grab(winit::window::CursorGrabMode::Locked);
            gpu.window.set_cursor_visible(false);

        } else {
            let _ = gpu.window.set_cursor_grab(winit::window::CursorGrabMode::None);
            gpu.window.set_cursor_visible(true);
        }

        self.mouse_delta = mouse_delta;
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        *self.keys.get(&key).unwrap_or(&false)
    }

    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        *self.key_just_pressed.get(&key).unwrap_or(&false)
    }

    pub fn is_mouse_button_pressed(&self, button: winit::event::MouseButton) -> bool {
        *self.mouse_buttons.get(&button).unwrap_or(&false)
    }

    pub fn is_mouse_button_just_pressed(&self, button: winit::event::MouseButton) -> bool {
        *self.mouse_buttons_just_pressed.get(&button).unwrap_or(&false)
    }

    pub fn get_mouse_delta(&self) -> (f64, f64) {
        self.mouse_delta
    }

    pub fn get_mouse_position(&self) -> (f64, f64) {
        self.prev_mouse_pos
    }
}
