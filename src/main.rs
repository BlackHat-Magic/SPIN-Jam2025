use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

pub mod render;
pub mod utils;

pub use ecs::*;
use render::Gpu;
pub use utils::*;

#[derive(Default)]
struct App {
    state: Option<Gpu>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        let state = pollster::block_on(Gpu::new(window.clone()));
        self.state = Some(state);

        window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let state = self.state.as_mut().unwrap();
        match event {
            WindowEvent::CloseRequested => {
                println!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                state.render();
                state.get_window().request_redraw();
            }
            WindowEvent::Resized(size) => {
                state.resize(size);
            }
            _ => (),
        }
    }
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();

    let _ = my_system;
}

#[derive(Component)]
struct Transform {
    position: f32,
}

#[derive(Component)]
struct Velocity(f32);

#[derive(Resource)]
struct Time {
    delta_seconds: f32,
}

ecs::system!(
    fn my_system(
        query: query (&mut Transform, &Velocity),
        time: res &Time,
    ) {
        if time.is_none() {
            return;
        }
        let time = time.unwrap();

        for (transform, velocity) in query {
            transform.position += velocity.0 * time.delta_seconds;
        }
    }
);
