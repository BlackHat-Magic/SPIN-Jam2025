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

pub mod audio;
pub mod physics;
pub mod render;
pub mod utils;

pub use audio::*;
pub use physics::*;
pub use render::model::ModelHandle;
use render::sprite::*;
pub use render::*;
use utils::input::Input;
pub use utils::time::*;
pub use utils::*;

#[derive(NetSend, Serialize, Deserialize)]
pub struct TestMessage {
    pub content: String,
}

fn test() -> anyhow::Result<()> {
    use rodio::*;
    let stream_handle = rodio::OutputStreamBuilder::open_default_stream()?;
    let sink = rodio::Sink::connect_new(stream_handle.mixer());

    let file = std::fs::File::open("resources/sounds/example.ogg")?;
    let source = rodio::Decoder::try_from(file)?.buffered();
    println!("Duration: {:?}", source.total_duration());
    sink.append(source);

    sink.sleep_until_end();

    Ok(())
}

#[tokio::main]
async fn main() {
    //test().unwrap();
    let mut app = App::new();

    struct WinitApp {
        app: App,
    }

    impl ApplicationHandler for WinitApp {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let window_attributes = Window::default_attributes()
                .with_title("Game")
                .with_visible(true)
                .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
                .with_position(winit::dpi::LogicalPosition::new(100, 100));
            let window = event_loop.create_window(window_attributes).unwrap();

            let gpu = pollster::block_on(Gpu::new(Arc::new(window)));
            self.app.insert_resource(gpu);

            let plugins = plugin_group!(
                physics::PhysicsPlugin,
                render::RenderPlugin,
                audio::AudioPlugin,
                utils::UtilPlugin::client(),
                networking::NetworkingPlugin::client(),
            );

            self.app.add_plugin(plugins);

            self.app.add_system(display_sprite, SystemStage::Update);
            self.app.add_system(control_player, SystemStage::Update);
            self.app.add_system(spin, SystemStage::Update);
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
        audio: res &Audio,
        commands: commands,
    ) {
        let (Some(gpu), Some(images), Some(audio)) = (gpu, images, audio) else {
            return;
        };

        audio.play("example", 0.2, true);

        let sprite = commands.spawn_entity();
        commands.add_component(sprite, SpriteBuilder::default().build(gpu, images));

        let entity = commands.spawn_entity();
        commands.add_component(entity, Transform::default());
        commands.add_component(entity, ModelHandle { path: "sphere".into() });
        commands.add_component(entity, MaterialHandle { name: "test_mat".into() });

        let cube = commands.spawn_entity();
        commands.add_component(cube, Transform {
            pos: Vec3::new(2.0, 0.0, 0.0),
            ..Default::default()
        });
        commands.add_component(cube, ModelHandle { path: "cube".into() });
        commands.add_component(cube, MaterialHandle { name: "test_mat".into() });

        use rand::prelude::*;
        let mut rng = rand::rng();

        for _ in 0..50 {
            let pos = Vec3::new(
                rng.random_range(-50.0..=50.0),
                0.0,
                rng.random_range(-50.0..=50.0),
            );

            let cube = commands.spawn_entity();
            commands.add_component(cube, Transform {
                pos,
                ..Default::default()
            });
            commands.add_component(cube, ModelHandle { path: "cube".into() });
            commands.add_component(cube, MaterialHandle { name: "test_mat".into() });
        }

        for _ in 0..50 {
            let pos = Vec3::new(
                rng.random_range(-50.0..=50.0),
                0.0,
                rng.random_range(-50.0..=50.0),
            );

            let cube = commands.spawn_entity();
            commands.add_component(cube, Transform {
                pos,
                ..Default::default()
            });
            commands.add_component(cube, ModelHandle { path: "sphere".into() });
            commands.add_component(cube, MaterialHandle { name: "test_mat".into() });
        }

        for i in 0..50 {
            let light = commands.spawn_entity();
            commands.add_component(light, Transform {
                pos: Vec3::new(rng.random_range(-50.0..=50.0), 5.0, rng.random_range(-50.0..=50.0)),
                ..Default::default()
            });

            let hue = rng.random_range((-std::f32::consts::PI)..=std::f32::consts::PI);
            let saturation = 1.0;
            let value = 1.0;

            fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
                let c = v * s;
                let x = c * (1.0 - ((h / (std::f32::consts::PI / 3.0)).rem_euclid(2.0) - 1.0).abs());
                let m = v - c;

                let (r1, g1, b1) = if h < std::f32::consts::PI / 3.0 {
                    (c, x, 0.0)
                } else if h < 2.0 * std::f32::consts::PI / 3.0 {
                    (x, c, 0.0)
                } else if h < std::f32::consts::PI {
                    (0.0, c, x)
                } else if h < 4.0 * std::f32::consts::PI / 3.0 {
                    (0.0, x, c)
                } else if h < 5.0 * std::f32::consts::PI / 3.0 {
                    (x, 0.0, c)
                } else {
                    (c, 0.0, x)
                };

                Vec3::new(r1 + m, g1 + m, b1 + m)
            }

            let color = hsv_to_rgb(hue, saturation, value) * 10.0;

            commands.add_component(light, Light {
                brightness: color,
            });
        }

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
            1000.0,
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
    fn spin(
        time: res &Time,
        objects: query (&mut Transform, &ModelHandle),
    ) {
        let Some(time) = time else {
            return;
        };

        let delta = time.delta_seconds;
        for (transform, _) in objects {
            transform.rot = (Quat::from_axis_angle(Vec3::Y, delta) * transform.rot).normalize();
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


        let mut forward = player_transform.rot * -Vec3::Z;
        forward.y = 0.0;
        forward = forward.normalize();

        let mut right = player_transform.rot * Vec3::X;
        right.y = 0.0;
        right = right.normalize();

        let up = Vec3::Y;

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

        if input.is_key_pressed(winit::keyboard::KeyCode::KeyE) {
            movement += up;
        }
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyQ) {
            movement -= up;
        }

        // uses `length_squared` to avoid a square root calculation
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
