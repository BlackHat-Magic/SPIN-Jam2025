use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

pub use ecs::*;

pub mod render;
pub mod utils;
mod input;
use input::Input;

use render::Gpu;
pub use utils::*;

fn main() {
    let mut app = App::new();

    app.add_system(input::input_system, SystemStage::PreUpdate);
    app.add_system(render::render_system, SystemStage::Render);
    app.add_system(render::init_shaders, SystemStage::Init);
    app.add_system(render::init_models, SystemStage::Init);

    struct WinitApp {
        app: App,
    }

    impl ApplicationHandler for WinitApp {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let window_attributes = Window::default_attributes()
                .with_title("Klaus of Death")
                .with_visible(true)
                .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
                .with_position(winit::dpi::LogicalPosition::new(100, 100));
            let window = event_loop
                .create_window(window_attributes)
                .unwrap();

            let gpu = pollster::block_on(Gpu::new(Arc::new(window)));
            self.app.insert_resource(gpu);

            self.app.init();
        }

        fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
            match event {
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                    self.app.de_init();
                }
                _ => {
                    let window_events = input::WindowEvents::new(Some(event));
                    self.app.insert_resource(window_events);
                    self.app.run();
                }
            }
        }

        fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
            // Request a redraw to keep the render loop going
            // We can't easily access the window here, so let's just run the app
            self.app.run();
        }
    }

    app.insert_resource(Input::new());
    
    let app = WinitApp { app };

    let event_loop = EventLoop::builder().build().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = app;
    event_loop.run_app(&mut app).expect("Failed to run event loop");
}
