use std::sync::Arc;

use std::sync::mpsc::{Sender, channel};
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

    app.add_system(init_window, SystemStage::Init);
    app.add_system(input::input_system, SystemStage::PreUpdate);
    app.add_system(render::render_system, SystemStage::Render);

    app.run();
}


system!(
    fn init_window(
        commands: commands
    ) {
        use winit::platform::x11::EventLoopBuilderExtX11;

        struct App {
            tx_event: Sender<WindowEvent>,
            tx_window: Sender<Window>,
        }

        impl ApplicationHandler for App {
            fn resumed(&mut self, event_loop: &ActiveEventLoop) {
                let window =
                    event_loop
                        .create_window(Window::default_attributes())
                        .unwrap();

                self.tx_window.send(window).unwrap();
            }

            fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
                match event {
                    WindowEvent::CloseRequested => {
                        event_loop.exit();
                    }
                    _ => (),
                }

                self.tx_event.send(event).unwrap();
            }
        }

        let (tx_event, rx_event) = channel();
        commands.insert_resource(Input::new(rx_event));
        
        let (tx_window, rx_window) = channel();
        let app = App { tx_event, tx_window };

        std::thread::spawn(move || {
            let event_loop = EventLoop::builder().with_any_thread(true).build().expect("Failed to create event loop");
            event_loop.set_control_flow(ControlFlow::Wait);

            let mut app = app;
            event_loop.run_app(&mut app).expect("Failed to run event loop");
        });

        let window = Arc::new(rx_window.recv().unwrap());
        let gpu = pollster::block_on(Gpu::new(window.clone()));

        commands.insert_resource(gpu);
    }
);
