use std::sync::Arc;

use glam::*;
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

pub use ecs::*;
pub use networking::*;

pub mod physics;
pub mod render;
pub mod utils;

pub use physics::*;
pub use render::model::ModelHandle;
use render::sprite::*;
use render::*;
use utils::input::Input;
pub use utils::time::*;
pub use utils::*;

#[derive(NetSend, NetRecv, Serialize, Deserialize)]
pub struct TestMessage {
    pub content: String,
}

fn main() {
    let mut app = App::new();

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
            let window = event_loop.create_window(window_attributes).unwrap();

            let gpu = pollster::block_on(Gpu::new(Arc::new(window)));
            self.app.insert_resource(gpu);

            let default_plugins = plugin_group!(
                utils::UtilPlugin,
                physics::PhysicsPlugin,
                render::RenderPlugin,
                networking::NetworkingPlugin,
            );

            self.app.add_plugin(default_plugins);

            self.app.add_system(update_time, SystemStage::PreUpdate);
            self.app.add_system(display_sprite, SystemStage::Update);
            self.app.add_system(control_player, SystemStage::Update);
            self.app.add_system(init_scene, SystemStage::Init);

            self.app.init();
            self.app.run();
        }

        fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            _id: WindowId,
            event: WindowEvent,
        ) {
            match event {
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                    self.app.de_init();
                }
                WindowEvent::RedrawRequested => {
                    self.app.run();
                }
                _ => {
                    let window_events = self.app.get_resource_mut::<input::WindowEvents>();
                    if let Some(window_events) = window_events {
                        window_events.events.push(event.clone());
                    }
                }
            }
        }

        fn device_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _device_id: winit::event::DeviceId,
            event: winit::event::DeviceEvent,
        ) {
            let device_events = self.app.get_resource_mut::<input::DeviceEvents>();
            if let Some(device_events) = device_events {
                device_events.events.push(event);
            }
        }

        fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
            self.app.run();
        }
    }

    app.insert_resource(input::WindowEvents { events: Vec::new() });
    app.insert_resource(input::DeviceEvents { events: Vec::new() });

    let mut app = WinitApp { app };

    let event_loop = EventLoop::builder()
        .build()
        .expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    event_loop
        .run_app(&mut app)
        .expect("Failed to run event loop");

    // Makes call to std::process::exit to avoid double drop of resources
    std::process::exit(0);
}

system! {
    fn init_scene(
        images: res &Images,
        gpu: res &Gpu,
        commands: commands,
    ) {
        let (Some(gpu), Some(images)) = (gpu, images) else {
            return;
        };

        let sprite = commands.spawn_entity();
        commands.add_component(sprite, SpriteBuilder::default().build(gpu, images));

        let entity = commands.spawn_entity();
        commands.add_component(entity, Transform::default());
        commands.add_component(entity, ModelHandle { path: "sphere".into() });
        commands.add_component(entity, MaterialHandle { name: "test_mat".into() });

        let camera_entity = commands.spawn_entity();
        commands.add_component(camera_entity, Transform {
            pos: Vec3::new(0.0, 0.0, -5.0),
            rot: Quat::look_to_rh(Vec3::Z, Vec3::Y),
            ..Default::default()
        });

        commands.add_component(camera_entity, Camera::new(
            45.0_f32.to_radians(),
            800.0 / 600.0,
            0.1,
            100.0,
        ));
    }
}

system! {
    fn display_sprite(
        gpu: res &mut Gpu,
        sprites: query (&Sprite),
    ) {
        let Some(gpu) = gpu else {
            return;
        };

        for sprite in sprites {
            gpu.display(sprite, (100.0, 100.0), (4.0, 4.0), 0.0, Align::Center);
        }
    }
}

system! {
    fn control_player(
        input: res &mut Input,
        time: res &Time,
        player: query (&mut Transform, &Camera),
    ) {
        let Some(input) = input else {
            return;
        };

        let Some(time) = time else {
            return;
        };

        let Some((player_transform, _camera)) = player.next() else {
            return;
        };

        if input.is_mouse_button_just_pressed(winit::event::MouseButton::Left) {
            input.cursor_grabbed = true;
        }

        if input.is_key_just_pressed(winit::keyboard::KeyCode::Escape) {
            input.cursor_grabbed = false;
        }


        let forward = player_transform.rot * -Vec3::Z;
        let right = player_transform.rot * Vec3::X;

        let mut movement = Vec3::ZERO;

        if input.is_key_pressed(winit::keyboard::KeyCode::KeyW) {
            movement += forward;
        }
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyS) {
            movement -= forward;
        }

        if input.is_key_pressed(winit::keyboard::KeyCode::KeyA) {
            movement -= right;
        }
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyD) {
            movement += right;
        }

        if movement.length_squared() > 0.0 {
            movement = movement.normalize();
            movement = movement * 5.0 * time.delta_seconds;
            player_transform.pos += movement;
        }

        let (mouse_dx, mouse_dy) = input.get_mouse_delta();
        if input.cursor_grabbed && (mouse_dx != 0.0 || mouse_dy != 0.0) {
            let sensitivity = 0.0008;
            let yaw = -mouse_dx as f32 * sensitivity;
            let pitch = -mouse_dy as f32 * sensitivity;

            let cur_rot = player_transform.rot;
            let cur_euler = cur_rot.to_euler(EulerRot::YXZ);
            let new_pitch = (cur_euler.1 + pitch).clamp(-std::f32::consts::FRAC_PI_2 + 0.01, std::f32::consts::FRAC_PI_2 - 0.01);
            let pitch = new_pitch - cur_euler.1;

            let yaw_rot = Quat::from_axis_angle(Vec3::Y, yaw);
            let pitch_rot = Quat::from_axis_angle(right, pitch);
            player_transform.rot = (yaw_rot * pitch_rot * player_transform.rot).normalize();
        }
    }
}
