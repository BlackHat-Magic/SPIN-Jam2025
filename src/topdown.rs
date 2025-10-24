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
pub use render::*;
use utils::input::Input;
pub use utils::time::*;
pub use utils::*;

#[tokio::main]
async fn main() {
    let mut app = App::new();

    struct WinitApp {
        app: App,
    }

    impl ApplicationHandler for WinitApp {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let window_attributes = Window::default_attributes()
                .with_title("Game")
                .with_visible(true)
                .with_inner_size(winit::dpi::LogicalSize::new(1280, 720))
                .with_position(winit::dpi::LogicalPosition::new(100, 100));
            let window = event_loop.create_window(window_attributes).unwrap();

            let gpu = pollster::block_on(Gpu::new(Arc::new(window)));
            self.app.insert_resource(gpu);

            let plugins = plugin_group!(
                // physics::PhysicsPlugin,
                render::RenderPlugin,
                utils::UtilPlugin::client(),
                // networking::NetworkingPlugin::client(),
            );
            self.app.add_plugin(plugins);

            self.app.add_system(draw_sprites, SystemStage::Update);
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

        // I hath decided: 1 unit is 32 px
        let background = commands.spawn_entity();
        commands.add_component(background, SpriteBuilder {
            image_path: "clue_simplified".to_string(),
            w: 795,
            h: 795,
            ..Default::default()
        }.build(gpu, images));
        commands.add_component(background, Transform {
            pos: Vec3::new(0.0, 0.0, 0.0),
            rot: Quat::look_to_rh(Vec3::Z, Vec3::Y),
            scale: Vec3::new(0.8855, 0.8855, 0.0),
            ..Default::default()
        });
        commands.add_component(background, Rotation2D(0.0));

        // player
        let player = commands.spawn_entity();
        commands.add_component(player, SpriteBuilder {
            image_path: "rawr".to_string(),
            w: 32,
            h: 32,
            ..Default::default()
        }.build(gpu, images));
        commands.add_component(player, Transform {
            pos: Vec3::new(0.0, 0.0, 0.1),
            rot: Quat::look_to_rh(Vec3::Z, Vec3::Y),
            scale: Vec3::new(1.0, 1.0, 0.0),
            ..Default::default()
        });
        commands.add_component(player, Camera::new(
            45.0_f32.to_radians(),
            800.0 / 600.0,
            0.1,
            100.0,
        ));
        commands.add_component(player, Rotation2D(3.14 / 4.0));

        let enemy = commands.spawn_entity();
        commands.add_component(enemy, SpriteBuilder {
            image_path: "rawr".to_string(),
            w: 32,
            h: 32,
            ..Default::default()
        }.build(gpu, images));
        commands.add_component(enemy, Transform {
            pos: Vec3::new(4.0, 4.0, 0.2),
            rot: Quat::look_to_rh(Vec3::Z, Vec3::Y),
            scale: Vec3::new(1.0, 1.0, 0.0),
            ..Default::default()
        });
        commands.add_component(enemy, Rotation2D(0.0));
    }
}

system! {
    fn draw_sprites(
        gpu: res &mut Gpu,
        sprites: query (&Sprite, &Transform, &Rotation2D),
        player: query (&Transform, &Camera)
    ) {
        let Some(gpu) = gpu else {return;};
        let Some((player_transform, _camera)) = player.next() else {return;};

        for (sprite, transform, rotation) in sprites {
            let relative_x = transform.pos.x - player_transform.pos.x;
            let relative_y = transform.pos.y - player_transform.pos.y;
            let z_index = transform.pos.z;
            let x_px = relative_x * 32.0 + 640.0;
            let y_px = relative_y * 32.0 + 360.0;
            gpu.display(sprite,
                (x_px, y_px),
                (transform.scale.x, transform.scale.y),
                rotation.0,
                z_index,
                Align::Center
            );
        }
    }
}

system! {
    fn control_player(
        input: res &mut Input,
        time: res &Time,
        player: query (&mut Transform, &Camera, &mut Rotation2D),
    ) {
        let Some (input) = input else {return;};
        let Some (time) = time else {return;};
        let Some((player_transform, _camera, rotation)) = player.next() else {return;};

        // WASD
        let mut movement = Vec3::ZERO;
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyW) {movement -= Vec3::Y;}
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyS) {movement += Vec3::Y;}
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyA) {movement -= Vec3::X;}
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyD) {movement += Vec3::X;}
        // uses `length_squared` to avoid a square root calculation
        if movement.length_squared() > 0.0 {
            movement = movement.normalize();
            movement = movement * 10.0 * time.delta_seconds;
            player_transform.pos += movement;
        }

        let (mousex, mousey) = input.get_mouse_position();
        let to_mousex = mousex - 640.0;
        let to_mousey = mousey - 360.0;
        rotation.0 = to_mousey.atan2(to_mousex) as f32;
    }
}