use klaus_of_death::physics::{BodyInit, Camera, PhysicsTestWorld, Transform};

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
