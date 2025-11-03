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
pub mod spin;
pub mod utils;

pub use physics::*;
pub use render::model::ModelHandle;
use render::sprite::*;
pub use render::*;
pub use spin::*;
use utils::input::Input;
pub use utils::time::*;
pub use utils::*;

static UNIT_SIZE: f32 = 32.0;
static SPRITE_SCALE: f32 = 2.0;
static PLAYER_SPEED: f32 = 16.0;
static SCREEN_W: u32 = 1280;
static SCREEN_H: u32 = 720;

static ENEMY_SPEED: f32 = 12.0;
static ENEMY_PERSONAL_SPACE: f32 = 2.0;
static ENEMY_VISION_DIST: f32 = 10.0;
static ENEMY_SUS_TIMER: f32 = 2.0;
static ENEMY_VISION_RADIANS: f32 = 1.0;
static ENEMY_SURPRISE_TIMER: f32 = 0.5;

fn ray_intersects_segment(ray_origin: Vec3, ray_dir: Vec3, ray_len: f32, wall: &Wall) -> bool {
    let wall_dir = wall.p2 - wall.p1;
    let denom = ray_dir.x * wall_dir.y - ray_dir.y * wall_dir.x;
    if denom.abs() < f32::EPSILON {
        return false;
    }

    let diff = wall.p1 - ray_origin;
    let t = (diff.x * wall_dir.y - diff.y * wall_dir.x) / denom;
    let s = (diff.x * ray_dir.y - diff.y * ray_dir.x) / denom;

    t >= 0.0 && t <= ray_len && s >= 0.0 && s <= 1.0
}

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
                .with_inner_size(winit::dpi::LogicalSize::new(SCREEN_W, SCREEN_H))
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

            self.app.add_system(update_animations, SystemStage::Update);
            self.app.add_system(draw_sprites, SystemStage::Update);
            self.app.add_system(draw_walls, SystemStage::Update);
            self.app.add_system(control_player, SystemStage::Update);
            self.app.add_system(process_ai, SystemStage::Update);
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
        let bg_sprite_size = 2048.0;
        let bg_tile_size = 22.0;
        let bg_scale = UNIT_SIZE * bg_tile_size / bg_sprite_size * SPRITE_SCALE;
        commands.add_component(background, SpriteBuilder {
            image_path: "map_placeholder".to_string(),
            w: bg_sprite_size as u32,
            h: bg_sprite_size as u32,
            ..Default::default()
        }.build(gpu, images));
        commands.add_component(background, Transform {
            pos: Vec3::new(0.0, 0.0, 0.0),
            rot: Quat::look_to_rh(Vec3::Z, Vec3::Y),
            scale: Vec3::new(bg_scale, bg_scale, 0.0),
            ..Default::default()
        });
        commands.add_component(background, Rotation2D(0.0));

        // player
        let player = commands.spawn_entity();
        let player_sprite_size = 256.0;
        let player_scale = UNIT_SIZE / player_sprite_size * SPRITE_SCALE;
        commands.add_component(player, SpriteBuilder {
            image_path: "player_placeholder".to_string(),
            w: player_sprite_size as u32,
            h: player_sprite_size as u32,
            ..Default::default()
        }.build(gpu, images));
        commands.add_component(player, Transform {
            pos: Vec3::new(0.0, 0.0, 0.1),
            rot: Quat::look_to_rh(Vec3::Z, Vec3::Y),
            scale: Vec3::new(player_scale, player_scale, 0.0),
            ..Default::default()
        });
        commands.add_component(player, Camera::new(
            45.0_f32.to_radians(),
            800.0 / 600.0,
            0.1,
            100.0,
        ));
        commands.add_component(player, Rotation2D(3.14 / 4.0));
        commands.insert_resource(PlayerPosition(Vec3::ZERO));

        let enemy = commands.spawn_entity();
        let enemy_sprite_size = 256.0;
        let enemy_scale = UNIT_SIZE / enemy_sprite_size * SPRITE_SCALE;
        commands.add_component(enemy, SpriteBuilder {
            image_path: "enemy_placeholder".to_string(),
            w: enemy_sprite_size as u32,
            h: enemy_sprite_size as u32,
            ..Default::default()
        }.build(gpu, images));
        commands.add_component(enemy, Transform {
            pos: Vec3::new(4.0, 4.0, 0.2),
            rot: Quat::look_to_rh(Vec3::Z, Vec3::Y),
            scale: Vec3::new(enemy_scale, enemy_scale, 0.0),
            ..Default::default()
        });
        commands.add_component(enemy, Rotation2D(0.0));
        commands.add_component(enemy, Ai {
            last_position: Vec3::ZERO,
            state: AIState::Idle,
        });

        // walls container
        let walls = commands.spawn_entity();
        let mut walls_comp = Walls(Vec::new());
        walls_comp.0.push(Wall {
            p1: Vec3::new(-9.0, -10.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-9.0, 2.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-9.0, -10.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(9.0, -10.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(9.0, -10.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(9.0, 7.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(9.0, 7.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(2.0, 7.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(2.0, 7.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(2.0, 9.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-2.0, 9.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-2.0, 7.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-2.0, 7.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-9.0, 7.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-9.0, 7.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-9.0, 4.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-9.0, -1.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-7.0, -1.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-5.0, -1.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-1.0, -1.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-4.0, -1.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-4.0, 7.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-2.0, -1.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-2.0, -3.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-2.0, -5.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-2.0, -8.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-2.0, -9.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(-2.0, -10.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(2.0, -1.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(2.0, -3.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(2.0, -5.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(2.0, -8.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(2.0, -9.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(2.0, -10.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(-2.0, -7.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(2.0, -7.0, 0.0) * SPRITE_SCALE
        });
        walls_comp.0.push(Wall {
            p1: Vec3::new(1.0, -1.0, 0.0) * SPRITE_SCALE,
            p2: Vec3::new(4.0, -1.0, 0.0) * SPRITE_SCALE
        });
        // TODO: why does this wall break everything?
        // walls_comp.0.push(Wall {
        //     p1: Vec3::new(4.0, -1.0, 0.0) * SPRITE_SCALE,
        //     p2: Vec3::new(4.0, 7.0, 0.0) * SPRITE_SCALE
        // });
        commands.add_component(walls, walls_comp);
        commands.add_component(walls, SpriteBuilder {
            image_path: "rawr".to_string(),
            w: UNIT_SIZE as u32,
            h: UNIT_SIZE as u32,
            ..Default::default()
        }.build(gpu, images));
    }
}

system! {
    fn draw_sprites(
        gpu: res &mut Gpu,
        sprites: query (&Sprite, &Transform, &Rotation2D),
        animations: query (&Animation, &Transform, &Rotation2D),
        player: query (&Transform, &Camera)
    ) {
        let Some(gpu) = gpu else {return;};
        let Some((player_transform, _camera)) = player.next() else {return;};

        for (sprite, transform, rotation) in sprites {
            let relative_x = transform.pos.x - player_transform.pos.x;
            let relative_y = transform.pos.y - player_transform.pos.y;
            let z_index = transform.pos.z;
            let x_px = relative_x * UNIT_SIZE + SCREEN_W as f32 / 2.0;
            let y_px = relative_y * UNIT_SIZE + SCREEN_H as f32 / 2.0;
            gpu.display(sprite,
                (x_px, y_px),
                (transform.scale.x, transform.scale.y),
                rotation.0,
                z_index,
                Align::Center
            );
        }

        for (animation, transform, rotation) in animations {
            let relative_x = transform.pos.x - player_transform.pos.x;
            let relative_y = transform.pos.y - player_transform.pos.y;
            let z_index = transform.pos.z;
            let x_px = relative_x * UNIT_SIZE + SCREEN_W as f32 / 2.0;
            let y_px = relative_y * UNIT_SIZE + SCREEN_H as f32 / 2.0;
            gpu.display(animation, (x_px, y_px), (transform.scale.x, transform.scale.y), rotation.0, z_index, Align::Center);
        }
    }
}

system! {
    fn process_ai(
        time: res &Time,
        player_pos: res &PlayerPosition,
        enemies: query (&mut Transform, &mut Rotation2D, &mut Ai),
        walls_comp: query (&Walls)
    ) {
        let Some(time) = time else {return;};
        let Some(player_pos) = player_pos else {return;};
        let Some(walls) = walls_comp.next() else {return;};

        for (enemy_transform, enemy_rotation, ai) in enemies {
            let displacement = player_pos.0 - enemy_transform.pos;
            let dist = displacement.length();
            let player_dir = displacement.normalize_or_zero();
            let facing_dir = Vec3::new(enemy_rotation.0.cos(), enemy_rotation.0.sin(), 0.0);

            // Helper: Check if player is visible (in range, FOV, clear LOS)
            let mut is_visible = |current_facing_dir: Vec3| -> bool {
                if dist > ENEMY_VISION_DIST || dist < f32::EPSILON { return false; }
                let dot = current_facing_dir.dot(player_dir);
                if dot.acos() > ENEMY_VISION_RADIANS { return false; }  // Out of FOV half-angle
                // Check LOS: ray to player
                for wall in walls.0.iter() {
                    if ray_intersects_segment(enemy_transform.pos, player_dir, dist, wall) {
                        return false;  // Hits wall before player
                    }
                }
                true
            };

            match ai.state {
                AIState::Idle => {
                    if dist < ENEMY_PERSONAL_SPACE {
                        println!("Too close; sus");
                        ai.state = AIState::Sus(ENEMY_SUS_TIMER);
                        continue;
                    }
                    if !is_visible(facing_dir) { continue; }  // Not visible: stay idle

                    println!("In vision cone and visible; sus");
                    ai.state = AIState::Sus(ENEMY_SUS_TIMER);
                    ai.last_position = player_pos.0;  // Record on detection
                }
                AIState::Sus(mut countdown) => {
                    if !is_visible(facing_dir) {  // Check current facing (pre-turn)
                        println!("Lost sight during sus");
                        ai.state = AIState::Idle;
                        ai.last_position = Vec3::ZERO;
                        continue;
                    }

                    // Still visible: turn to player
                    enemy_rotation.0 = displacement.y.atan2(displacement.x);
                    let facing_dir_after_turn = Vec3::new(enemy_rotation.0.cos(), enemy_rotation.0.sin(), 0.0);
                    if !is_visible(facing_dir_after_turn) {  // Double-check after turn (edge case)
                        ai.state = AIState::Idle;
                        continue;
                    }

                    ai.last_position = player_pos.0;
                    let mut dt = time.delta_seconds;
                    if dist < ENEMY_PERSONAL_SPACE { dt *= 2.0; }
                    countdown -= dt;
                    if countdown <= 0.0 {
                        println!("Sus timer out; noticed");
                        ai.state = AIState::Noticed(ENEMY_SURPRISE_TIMER);
                    } else {
                        ai.state = AIState::Sus(countdown);
                    }
                }
                AIState::Noticed(mut countdown) => {
                    let was_visible = is_visible(facing_dir);
                    if !was_visible {
                        println!("Lost sight during noticed");
                        ai.state = AIState::Search(ENEMY_SUS_TIMER);  // Or Idle; search last pos
                        continue;
                    }

                    // Turn to player (update facing for next frame's visible check)
                    enemy_rotation.0 = displacement.y.atan2(displacement.x);
                    ai.last_position = player_pos.0;

                    countdown -= time.delta_seconds;
                    if countdown <= 0.0 {
                        println!("Noticed timer out; chase");
                        ai.state = AIState::Chase(was_visible);  // Always true here, but for consistency
                    } else {
                        ai.state = AIState::Noticed(countdown);
                    }
                }
                AIState::Chase(initially_visible) => {
                    let target = if initially_visible && is_visible(facing_dir) {
                        player_pos.0  // Still see: chase current
                    } else {
                        ai.last_position  // Lost sight: chase last known
                    };
                    let chase_disp = target - enemy_transform.pos;
                    let chase_dist = chase_disp.length();
                    if chase_dist > f32::EPSILON {
                        let movement = chase_disp.normalize() * ENEMY_SPEED * time.delta_seconds;
                        enemy_rotation.0 = chase_disp.y.atan2(chase_disp.x);
                        enemy_transform.pos += movement;

                        // Optional: If reach last pos and not visible, go to Search or Idle
                        if !initially_visible && chase_dist < 0.5 {  // Threshold
                            ai.state = AIState::Search(3.0);  // Search for 3s
                        }
                    }
                }
                AIState::Search(mut countdown) => {
                    // Simple: face/linger at last pos, check if regain sight
                    if ai.last_position.length_squared() > 0.0 {
                        let search_disp = ai.last_position - enemy_transform.pos;
                        enemy_rotation.0 = search_disp.y.atan2(search_disp.x);
                    }
                    countdown -= time.delta_seconds;
                    if is_visible(facing_dir) {
                        ai.state = AIState::Sus(ENEMY_SUS_TIMER);  // Regain sight
                    } else if countdown <= 0.0 {
                        ai.state = AIState::Idle;
                        ai.last_position = Vec3::ZERO;
                    } else {
                        ai.state = AIState::Search(countdown);
                    }
                }
            }
        }
    }
}

system! {
    fn draw_walls(
        gpu: res &mut Gpu,
        player_pos: res &PlayerPosition,
        walls: query (&Walls, &Sprite),
    ) {
        let Some(gpu) = gpu else {return;};
        let Some(player_pos) = player_pos else {return;};
        let Some((walls_comp, walls_sprite)) = walls.next() else {return;};

        for wall in walls_comp.0.iter() {
            let wall_dir = wall.p2 - wall.p1;
            let wall_ctr = wall.p1 + wall_dir / 2.0;
            let ctr_rx = wall_ctr.x - player_pos.0.x;
            let ctr_ry = wall_ctr.y - player_pos.0.y;
            let x_px = ctr_rx * UNIT_SIZE + SCREEN_W as f32 / 2.0;
            let y_px = ctr_ry * UNIT_SIZE + SCREEN_H as f32 / 2.0;

            let mut scale_x = 0.1;
            let mut scale_y = 0.1;
            if wall_dir.x.abs() > wall_dir.y.abs() {
                scale_x = wall_dir.x.abs();
            } else {
                scale_y = wall_dir.y.abs();
            }
            gpu.display(walls_sprite,
                (x_px, y_px),
                (scale_x, scale_y),
                0.0,
                1.0,
                Align::Center
            )
        }
    }
}

system! {
    fn control_player(
        input: res &mut Input,
        time: res &Time,
        mut player_pos: res &mut PlayerPosition,
        player: query (&mut Transform, &Camera, &mut Rotation2D),
        walls: query (&Walls),
    ) {
        let Some (input) = input else {return;};
        let Some (time) = time else {return;};
        let Some(player_pos) = player_pos else {return;};
        let Some((player_transform, _camera, rotation)) = player.next() else {return;};
        let Some (walls_comp) = walls.next() else {return;};

        // WASD
        let mut movement = Vec3::ZERO;
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyW) {movement -= Vec3::Y;}
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyS) {movement += Vec3::Y;}
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyA) {movement -= Vec3::X;}
        if input.is_key_pressed(winit::keyboard::KeyCode::KeyD) {movement += Vec3::X;}
        movement = movement.normalize();

        // ray intersection
        for wall in walls_comp.0.iter() {
            if ray_intersects_segment(
                player_transform.pos,
                movement,
                PLAYER_SPEED * time.delta_seconds * 8.0, // magic number 👻
                wall
            ) {
                movement = Vec3::ZERO;
            }
        }

        // uses `length_squared` to avoid a square root calculation
        if movement.length_squared() > 0.0 {
            movement = movement * PLAYER_SPEED * time.delta_seconds;
            player_transform.pos += movement;
        }

        let (mousex, mousey) = input.get_mouse_position();
        let to_mousex = mousex - SCREEN_W as f64 / 2.0;
        let to_mousey = mousey - SCREEN_H as f64 / 2.0;
        rotation.0 = to_mousey.atan2(to_mousex) as f32;
        player_pos.0 = player_transform.pos;
    }
}
