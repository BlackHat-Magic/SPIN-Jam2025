use klaus_of_death::physics::{
    AngularVelocity, BodyInit, Camera, Collider, ForceAccumulator, PhysicsDebugSettings,
    PhysicsEvents, PhysicsPlugin, PhysicsTestWorld, PhysicsTime, PhysicsWorld, RigidBody,
    Transform, Velocity,
};
use klaus_of_death::{App, Commands, World};

use glam::{Mat4, Quat, Vec3};

fn assert_mat4_close(a: Mat4, b: Mat4, epsilon: f32) {
    let a = a.to_cols_array();
    let b = b.to_cols_array();
    for (ai, bi) in a.iter().zip(b.iter()) {
        assert!(
            (ai - bi).abs() <= epsilon,
            "matrices differ: {} vs {}",
            ai,
            bi
        );
    }

    #[test]
    fn physics_world_broad_phase_pairs_are_deterministic() {
        let mut app = App::new();
        app.add_plugin(PhysicsPlugin);

        let e1 = app.spawn_entity();
        app.add_component(e1, Transform::default()).unwrap();
        app.add_component(e1, RigidBody::dynamic(1.0)).unwrap();
        app.add_component(e1, Collider::sphere(0.5)).unwrap();
        app.add_component(e1, Velocity(Vec3::ZERO)).unwrap();
        app.add_component(e1, AngularVelocity(Vec3::ZERO)).unwrap();
        app.add_component(e1, ForceAccumulator(Vec3::ZERO)).unwrap();

        let mut t2 = Transform::default();
        t2.pos = Vec3::new(0.25, 0.0, 0.0);
        let e2 = app.spawn_entity();
        app.add_component(e2, t2).unwrap();
        app.add_component(e2, RigidBody::dynamic(1.0)).unwrap();
        app.add_component(e2, Collider::sphere(0.5)).unwrap();
        app.add_component(e2, Velocity(Vec3::ZERO)).unwrap();
        app.add_component(e2, AngularVelocity(Vec3::ZERO)).unwrap();
        app.add_component(e2, ForceAccumulator(Vec3::ZERO)).unwrap();

        let mut t3 = Transform::default();
        t3.pos = Vec3::new(5.0, 0.0, 0.0);
        let e3 = app.spawn_entity();
        app.add_component(e3, t3).unwrap();
        app.add_component(e3, RigidBody::dynamic(1.0)).unwrap();
        app.add_component(e3, Collider::sphere(0.5)).unwrap();
        app.add_component(e3, Velocity(Vec3::ZERO)).unwrap();
        app.add_component(e3, AngularVelocity(Vec3::ZERO)).unwrap();
        app.add_component(e3, ForceAccumulator(Vec3::ZERO)).unwrap();

        app.run();

        let commands: &Commands = &app;
        let world_ptr = commands.world;
        let physics_world = unsafe { World::get_resource::<PhysicsWorld>(world_ptr).unwrap() };

        let pairs = physics_world.broad_phase_pairs();
        assert_eq!(pairs, &[(e1, e2)]);

        // Running again without changes should yield the same ordering.
        app.run();
        let physics_world = unsafe { World::get_resource::<PhysicsWorld>(world_ptr).unwrap() };
        assert_eq!(physics_world.broad_phase_pairs(), &[(e1, e2)]);
    }

    #[test]
    fn physics_world_broad_phase_pairs_respect_axis_ordering() {
        let mut app = App::new();
        app.add_plugin(PhysicsPlugin);

        let mut transforms = [
            Vec3::new(-1.5, 0.0, 0.0),
            Vec3::new(-0.5, 0.0, 0.0),
            Vec3::new(0.5, 0.0, 0.0),
            Vec3::new(1.5, 0.0, 0.0),
        ];

        let mut entities = Vec::new();
        for pos in transforms.iter_mut() {
            let mut t = Transform::default();
            t.pos = *pos;
            let entity = app.spawn_entity();
            app.add_component(entity, t).unwrap();
            app.add_component(entity, RigidBody::dynamic(1.0)).unwrap();
            app.add_component(entity, Collider::sphere(0.75)).unwrap();
            app.add_component(entity, Velocity(Vec3::ZERO)).unwrap();
            app.add_component(entity, AngularVelocity(Vec3::ZERO))
                .unwrap();
            app.add_component(entity, ForceAccumulator(Vec3::ZERO))
                .unwrap();
            entities.push(entity);
        }

        app.run();

        let commands: &Commands = &app;
        let world_ptr = commands.world;
        let physics_world = unsafe { World::get_resource::<PhysicsWorld>(world_ptr).unwrap() };

        let expected_pairs = vec![
            (entities[0].min(entities[1]), entities[0].max(entities[1])),
            (entities[1].min(entities[2]), entities[1].max(entities[2])),
            (entities[2].min(entities[3]), entities[2].max(entities[3])),
        ];

        assert_eq!(physics_world.broad_phase_pairs(), expected_pairs);
    }

    #[test]
    fn physics_events_emit_broad_phase_pairs() {
        let mut app = App::new();
        app.add_plugin(PhysicsPlugin);

        let mut t1 = Transform::default();
        t1.pos = Vec3::new(-0.25, 0.0, 0.0);
        let e1 = app.spawn_entity();
        app.add_component(e1, t1).unwrap();
        app.add_component(e1, RigidBody::dynamic(1.0)).unwrap();
        app.add_component(e1, Collider::sphere(0.6)).unwrap();
        app.add_component(e1, Velocity(Vec3::ZERO)).unwrap();
        app.add_component(e1, AngularVelocity(Vec3::ZERO)).unwrap();
        app.add_component(e1, ForceAccumulator(Vec3::ZERO)).unwrap();

        let mut t2 = Transform::default();
        t2.pos = Vec3::new(0.25, 0.0, 0.0);
        let e2 = app.spawn_entity();
        app.add_component(e2, t2).unwrap();
        app.add_component(e2, RigidBody::dynamic(1.0)).unwrap();
        app.add_component(e2, Collider::sphere(0.6)).unwrap();
        app.add_component(e2, Velocity(Vec3::ZERO)).unwrap();
        app.add_component(e2, AngularVelocity(Vec3::ZERO)).unwrap();
        app.add_component(e2, ForceAccumulator(Vec3::ZERO)).unwrap();

        app.run();

        let commands: &Commands = &app;
        let world_ptr = commands.world;
        let events = unsafe { World::get_resource::<PhysicsEvents>(world_ptr).unwrap() };
        assert_eq!(events.broad_phase_pairs, vec![(e1.min(e2), e1.max(e2))]);
    }
}

#[test]
fn transform_matrix_roundtrip() {
    let transform = Transform {
        pos: Vec3::new(1.0, 2.0, 3.0),
        scale: Vec3::new(2.0, 3.0, 4.0),
        rot: Quat::from_euler(glam::EulerRot::XYZ, 0.3, -1.2, 0.7),
    };

    let matrix = transform.to_matrix();
    let expected =
        Mat4::from_scale_rotation_translation(transform.scale, transform.rot, transform.pos);
    assert_mat4_close(matrix, expected, 1e-6);

    let reconstructed = Transform::from_matrix(matrix);
    assert!(transform.pos.abs_diff_eq(reconstructed.pos, 1e-5));
    assert!(transform.scale.abs_diff_eq(reconstructed.scale, 1e-5));
    assert!(transform.rot.abs_diff_eq(reconstructed.rot, 1e-5));
}

#[test]
fn transform_view_matrix_is_inverse_of_model_matrix() {
    let transform = Transform {
        pos: Vec3::new(-5.0, 0.5, 12.0),
        scale: Vec3::ONE,
        rot: Quat::from_rotation_y(0.75),
    };

    let model = transform.to_matrix();
    let view = transform.to_view_matrix();
    let expected_view = model.inverse();

    assert_mat4_close(view, expected_view, 1e-5);
}

#[test]
fn camera_projection_matches_glam_helpers() {
    let camera = Camera::new(55.0_f32.to_radians(), 1920.0 / 1080.0, 0.01, 250.0);
    let projection = camera.projection_matrix();
    let expected = Mat4::perspective_rh(camera.fov_y, camera.aspect, camera.near, camera.far);
    assert_mat4_close(projection, expected, 1e-6);
}

#[test]
fn physics_test_world_initializes_with_defaults() {
    let world = PhysicsTestWorld::new();

    assert_eq!(world.gravity(), Vec3::new(0.0, -9.81, 0.0));
    assert!((world.dt() - (1.0 / 60.0)).abs() < f32::EPSILON);
    assert_eq!(world.body_count(), 0);
}

#[test]
fn physics_test_world_adds_bodies_and_steps() {
    let mut world = PhysicsTestWorld::new();

    let handle = world.add_body(BodyInit {
        position: Vec3::new(0.0, 1.0, 0.0),
        velocity: Vec3::ZERO,
        mass: 2.0,
    });

    assert_eq!(world.body_count(), 1);

    world.step(10);

    let state = world.body_state(handle).expect("body should exist");

    assert!(
        state.velocity.y < 0.0,
        "gravity should accelerate body downward"
    );
    assert!(state.position.y < 1.0, "body should have moved downward");
}

#[test]
fn physics_test_world_energy_helpers_track_system_energy() {
    let mut world = PhysicsTestWorld::new().with_gravity(Vec3::ZERO);

    world.add_body(BodyInit {
        position: Vec3::new(0.0, 0.0, 0.0),
        velocity: Vec3::new(1.0, 0.0, 0.0),
        mass: 3.0,
    });

    let kinetic = world.total_kinetic_energy();
    let potential = world.total_potential_energy();
    let total = world.total_energy();

    assert!(kinetic > 0.0);
    assert_eq!(potential, 0.0);
    assert!((total - kinetic).abs() < 1e-6);

    world.clear_bodies();
    assert_eq!(world.body_count(), 0);
    assert_eq!(world.total_energy(), 0.0);
}

#[test]
fn physics_test_world_seed_controls_randomized_bodies() {
    let mut world_a = PhysicsTestWorld::new().with_seed(42);
    let mut world_b = PhysicsTestWorld::new().with_seed(42);
    let mut world_c = PhysicsTestWorld::new().with_seed(1337);

    let handle_a1 = world_a.spawn_random_body();
    let handle_b1 = world_b.spawn_random_body();
    let handle_c1 = world_c.spawn_random_body();

    let state_a1 = world_a.body_state(handle_a1).unwrap();
    let state_b1 = world_b.body_state(handle_b1).unwrap();
    let state_c1 = world_c.body_state(handle_c1).unwrap();

    assert_eq!(state_a1.position, state_b1.position);
    assert_eq!(state_a1.velocity, state_b1.velocity);
    assert_eq!(state_a1.mass, state_b1.mass);

    assert_ne!(state_a1.position, state_c1.position);
    assert_ne!(state_a1.velocity, state_c1.velocity);

    world_b.reseed(9001);
    let handle_b2 = world_b.spawn_random_body();
    let state_b2 = world_b.body_state(handle_b2).unwrap();

    assert_ne!(state_b1.position, state_b2.position);
}

#[test]
fn physics_plugin_inserts_resources() {
    let mut app = App::new();
    app.add_plugin(PhysicsPlugin);

    let commands: &Commands = &app;
    let world_ptr = commands.world;

    let physics_world =
        unsafe { World::get_resource::<PhysicsWorld>(world_ptr).expect("PhysicsWorld missing") };
    assert_eq!(physics_world.gravity(), Vec3::new(0.0, -9.81, 0.0));
    assert_eq!(physics_world.body_count(), 0);

    unsafe {
        let _time = World::get_resource::<PhysicsTime>(world_ptr).expect("PhysicsTime missing");
        let _events =
            World::get_resource::<PhysicsEvents>(world_ptr).expect("PhysicsEvents missing");
        let _debug = World::get_resource::<PhysicsDebugSettings>(world_ptr)
            .expect("PhysicsDebugSettings missing");
    }
}

#[test]
fn physics_plugin_collects_bodies_from_ecs() {
    let mut app = App::new();
    app.add_plugin(PhysicsPlugin);

    let dynamic_entity = app.spawn_entity();
    app.add_component(dynamic_entity, RigidBody::dynamic(2.0))
        .unwrap();
    app.add_component(dynamic_entity, Collider::sphere(0.5))
        .unwrap();
    app.add_component(dynamic_entity, Transform::default())
        .unwrap();
    app.add_component(dynamic_entity, Velocity(Vec3::new(0.0, 1.0, 0.0)))
        .unwrap();
    app.add_component(dynamic_entity, AngularVelocity(Vec3::ZERO))
        .unwrap();
    app.add_component(dynamic_entity, ForceAccumulator(Vec3::ZERO))
        .unwrap();

    let static_entity = app.spawn_entity();
    app.add_component(static_entity, RigidBody::static_body())
        .unwrap();
    app.add_component(static_entity, Collider::cuboid(Vec3::splat(1.0)))
        .unwrap();
    app.add_component(static_entity, Transform::default())
        .unwrap();
    app.add_component(static_entity, Velocity(Vec3::ZERO))
        .unwrap();
    app.add_component(static_entity, AngularVelocity(Vec3::ZERO))
        .unwrap();
    app.add_component(static_entity, ForceAccumulator(Vec3::ZERO))
        .unwrap();

    {
        let commands: &Commands = &app;
        let world_ptr = commands.world;
        let time = unsafe {
            World::get_resource_mut::<PhysicsTime>(world_ptr).expect("PhysicsTime missing")
        };
        let dt = time.fixed_delta;
        time.accumulate(dt);
    }

    app.run();

    let commands: &Commands = &app;
    let world_ptr = commands.world;

    let physics_world =
        unsafe { World::get_resource::<PhysicsWorld>(world_ptr).expect("PhysicsWorld missing") };
    assert_eq!(physics_world.body_count(), 2);

    let dynamic_body = physics_world
        .get_body(dynamic_entity)
        .expect("dynamic body missing");
    assert!(!dynamic_body.rigid_body.is_static());
    assert_eq!(dynamic_body.accumulated_force, Vec3::ZERO);

    let static_body = physics_world
        .get_body(static_entity)
        .expect("static body missing");
    assert!(static_body.rigid_body.is_static());

    assert!(
        physics_world
            .bodies()
            .iter()
            .any(|body| matches!(body.collider, Collider::Sphere { .. }))
    );
    assert!(
        physics_world
            .bodies()
            .iter()
            .any(|body| matches!(body.collider, Collider::Box { .. }))
    );
}

#[test]
fn physics_plugin_applies_gravity_and_forces() {
    let mut app = App::new();
    app.add_plugin(PhysicsPlugin);

    let entity = app.spawn_entity();
    app.add_component(entity, RigidBody::dynamic(2.0)).unwrap();
    app.add_component(entity, Collider::sphere(0.25)).unwrap();
    app.add_component(entity, Transform::default()).unwrap();
    app.add_component(entity, Velocity(Vec3::ZERO)).unwrap();
    app.add_component(entity, AngularVelocity(Vec3::new(0.0, 1.0, 0.0)))
        .unwrap();
    app.add_component(entity, ForceAccumulator(Vec3::new(4.0, 0.0, 0.0)))
        .unwrap();

    unsafe {
        let commands: &Commands = &app;
        let world_ptr = commands.world;
        let time = World::get_resource_mut::<PhysicsTime>(world_ptr).expect("PhysicsTime missing");
        time.accumulate(time.fixed_delta);
    }

    app.run();

    let commands: &Commands = &app;
    let world_ptr = commands.world;

    unsafe {
        let dt = World::get_resource::<PhysicsTime>(world_ptr)
            .expect("PhysicsTime missing")
            .fixed_delta;

        let velocity = World::get_components::<Velocity>(world_ptr)
            .into_iter()
            .find(|(id, _)| *id == entity)
            .map(|(_, vel)| vel.0)
            .expect("Velocity component missing");

        let transform = World::get_components::<Transform>(world_ptr)
            .into_iter()
            .find(|(id, _)| *id == entity)
            .map(|(_, t)| t)
            .expect("Transform missing");

        let force_after_step = World::get_components::<ForceAccumulator>(world_ptr)
            .into_iter()
            .find(|(id, _)| *id == entity)
            .map(|(_, force)| force.0)
            .expect("ForceAccumulator missing");

        assert!((velocity.y + 9.81 * dt).abs() < 1e-5);
        assert!((velocity.x - 2.0 * dt).abs() < 1e-5);
        assert!(force_after_step.length() < 1e-5);
        assert!(transform.pos.y < 0.0);
    }
}
