use crate::*;
use glam::{Mat4, Quat, Vec3};
use std::{cmp::Ordering, collections::HashMap};

pub mod test;

pub use test::{BodyHandle, BodyInit, BodyState, PhysicsTestWorld};

const DEFAULT_GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);
const DEFAULT_FIXED_DT: f32 = 1.0 / 60.0;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PhysicsWorld::default());
        app.insert_resource(PhysicsTime::default());
        app.insert_resource(PhysicsEvents::default());
        app.insert_resource(PhysicsDebugSettings::default());

        app.add_system(sync_ecs_to_physics, SystemStage::PreUpdate);
        app.add_system(run_physics_step, SystemStage::Update);
        app.add_system(sync_physics_to_ecs, SystemStage::PostUpdate);
        app.add_system(emit_physics_events, SystemStage::PostUpdate);
    }
}

#[derive(Component)]
pub struct Transform {
    pub pos: Vec3,
    pub scale: Vec3,
    pub rot: Quat,
}

#[derive(Component)]
pub struct Rotation2D (pub f32);

impl Default for Transform {
    fn default() -> Self {
        Self {
            pos: Vec3::ZERO,
            scale: Vec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            rot: Quat::look_to_rh(-Vec3::Z, Vec3::Y),
        }
    }
}

impl Transform {
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rot, self.pos)
    }

    pub fn from_matrix(mat: Mat4) -> Self {
        let (scale, rot, pos) = mat.to_scale_rotation_translation();
        Self { pos, scale, rot }
    }

    pub fn to_view_matrix(&self) -> Mat4 {
        let translation = Mat4::from_translation(-self.pos);
        let rotation = Mat4::from_quat(self.rot.conjugate());
        rotation * translation
    }
}

#[derive(Component)]
pub struct Camera {
    pub fov_y: f32,
    pub aspect: f32,
    pub near: f32,
    pub far: f32,
}

impl Camera {
    pub fn new(fov_y: f32, aspect: f32, near: f32, far: f32) -> Self {
        Self {
            fov_y,
            aspect,
            near,
            far,
        }
    }

    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fov_y, self.aspect, self.near, self.far)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BodyType {
    Dynamic,
    Static,
}

#[derive(Component, Clone, Debug)]
pub struct RigidBody {
    pub body_type: BodyType,
    pub mass: f32,
}

impl RigidBody {
    pub fn dynamic(mass: f32) -> Self {
        let mass = mass.max(f32::EPSILON);
        Self {
            body_type: BodyType::Dynamic,
            mass,
        }
    }

    pub fn static_body() -> Self {
        Self {
            body_type: BodyType::Static,
            mass: f32::INFINITY,
        }
    }

    pub fn inverse_mass(&self) -> f32 {
        match self.body_type {
            BodyType::Dynamic => 1.0 / self.mass,
            BodyType::Static => 0.0,
        }
    }

    pub fn is_static(&self) -> bool {
        matches!(self.body_type, BodyType::Static)
    }
}

#[derive(Component, Clone, Debug)]
pub enum Collider {
    Sphere { radius: f32 },
    Box { half_extents: Vec3 },
    Capsule { half_height: f32, radius: f32 },
}

impl Collider {
    pub fn sphere(radius: f32) -> Self {
        Self::Sphere { radius }
    }

    pub fn cuboid(half_extents: Vec3) -> Self {
        Self::Box { half_extents }
    }

    pub fn capsule(half_height: f32, radius: f32) -> Self {
        Self::Capsule {
            half_height,
            radius,
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Velocity(pub Vec3);

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AngularVelocity(pub Vec3);

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ForceAccumulator(pub Vec3);

#[derive(Component, Clone, Debug, Default)]
pub struct PhysicsMaterial {
    pub restitution: f32,
    pub friction: f32,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Sleeping(pub bool);

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PhysicsProxy;

#[derive(Clone, Debug)]
pub struct PhysicsBody {
    pub entity: u32,
    pub rigid_body: RigidBody,
    pub collider: Collider,
    pub velocity: Velocity,
    pub angular_velocity: AngularVelocity,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
    pub accumulated_force: Vec3,
}

impl PhysicsBody {
    fn new(
        entity: u32,
        rigid_body: RigidBody,
        collider: Collider,
        transform: &Transform,
        velocity: Velocity,
        angular_velocity: AngularVelocity,
        accumulated_force: Vec3,
    ) -> Self {
        Self {
            entity,
            rigid_body,
            collider,
            velocity,
            angular_velocity,
            position: transform.pos,
            rotation: transform.rot,
            scale: transform.scale,
            accumulated_force,
        }
    }

    fn aabb(&self) -> (Vec3, Vec3) {
        match &self.collider {
            Collider::Sphere { radius } => {
                let r = radius.abs() * self.scale.max_element();
                let extents = Vec3::splat(r);
                (self.position - extents, self.position + extents)
            }
            Collider::Box { half_extents } => {
                let extents = Vec3::new(
                    half_extents.x * self.scale.x.abs(),
                    half_extents.y * self.scale.y.abs(),
                    half_extents.z * self.scale.z.abs(),
                );
                (self.position - extents, self.position + extents)
            }
            Collider::Capsule {
                half_height,
                radius,
            } => {
                let radial = radius.abs();
                let half_height = half_height.abs();
                let extents = Vec3::new(
                    radial * self.scale.x.abs(),
                    (half_height + radial) * self.scale.y.abs(),
                    radial * self.scale.z.abs(),
                );
                (self.position - extents, self.position + extents)
            }
        }
    }
}

#[inline]
fn aabb_overlap(min_a: Vec3, max_a: Vec3, min_b: Vec3, max_b: Vec3) -> bool {
    !(max_a.x < min_b.x
        || max_b.x < min_a.x
        || max_a.y < min_b.y
        || max_b.y < min_a.y
        || max_a.z < min_b.z
        || max_b.z < min_a.z)
}

#[derive(Resource, Debug)]
pub struct PhysicsWorld {
    gravity: Vec3,
    bodies: Vec<PhysicsBody>,
    entity_map: HashMap<u32, usize>,
    broad_phase_pairs: Vec<(u32, u32)>,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        Self {
            gravity: DEFAULT_GRAVITY,
            bodies: Vec::new(),
            entity_map: HashMap::new(),
            broad_phase_pairs: Vec::new(),
        }
    }
}

impl PhysicsWorld {
    pub fn new(gravity: Vec3) -> Self {
        Self {
            gravity,
            bodies: Vec::new(),
            entity_map: HashMap::new(),
            broad_phase_pairs: Vec::new(),
        }
    }

    pub fn gravity(&self) -> Vec3 {
        self.gravity
    }

    pub fn set_gravity(&mut self, gravity: Vec3) {
        self.gravity = gravity;
    }

    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    pub fn get_body(&self, entity: u32) -> Option<&PhysicsBody> {
        self.entity_map
            .get(&entity)
            .and_then(|index| self.bodies.get(*index))
    }

    pub fn bodies(&self) -> &[PhysicsBody] {
        &self.bodies
    }

    pub fn broad_phase_pairs(&self) -> &[(u32, u32)] {
        &self.broad_phase_pairs
    }

    fn clear(&mut self) {
        self.bodies.clear();
        self.entity_map.clear();
        self.broad_phase_pairs.clear();
    }

    fn add_body(&mut self, body: PhysicsBody) {
        let index = self.bodies.len();
        self.entity_map.insert(body.entity, index);
        self.bodies.push(body);
    }

    fn rebuild_broad_phase(&mut self) {
        self.broad_phase_pairs.clear();

        let mut entries: Vec<_> = self
            .bodies
            .iter()
            .map(|body| {
                let (min, max) = body.aabb();
                (min, max, body.entity)
            })
            .collect();

        entries.sort_by(|a, b| {
            a.0.x
                .partial_cmp(&b.0.x)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.2.cmp(&b.2))
        });

        for i in 0..entries.len() {
            let (min_a, max_a, ent_a) = entries[i];
            for j in (i + 1)..entries.len() {
                let (min_b, max_b, ent_b) = entries[j];
                if min_b.x > max_a.x {
                    break;
                }

                if aabb_overlap(min_a, max_a, min_b, max_b) {
                    let pair = if ent_a < ent_b {
                        (ent_a, ent_b)
                    } else {
                        (ent_b, ent_a)
                    };
                    self.broad_phase_pairs.push(pair);
                }
            }
        }

        self.broad_phase_pairs.sort();
        self.broad_phase_pairs.dedup();
    }
}

#[derive(Resource, Debug)]
pub struct PhysicsTime {
    pub fixed_delta: f32,
    accumulator: f32,
}

impl Default for PhysicsTime {
    fn default() -> Self {
        Self {
            fixed_delta: DEFAULT_FIXED_DT,
            accumulator: 0.0,
        }
    }
}

impl PhysicsTime {
    pub fn accumulate(&mut self, dt: f32) {
        self.accumulator += dt;
    }

    pub fn consume_step(&mut self) -> bool {
        if self.accumulator >= self.fixed_delta {
            self.accumulator -= self.fixed_delta;
            true
        } else {
            false
        }
    }
}

#[derive(Default, Resource, Debug)]
pub struct PhysicsEvents {
    pub contacts: Vec<PhysicsContactEvent>,
    pub broad_phase_pairs: Vec<(u32, u32)>,
}

#[derive(Clone, Debug, Default)]
pub struct PhysicsContactEvent {
    pub entity_a: u32,
    pub entity_b: u32,
}

#[derive(Default, Resource, Debug)]
pub struct PhysicsDebugSettings {
    pub show_contacts: bool,
}

system!(
    fn sync_ecs_to_physics(
        physics_world: res &mut PhysicsWorld,
        bodies: query (
            &EntityId,
            &Transform,
            &RigidBody,
            &Collider,
            &Velocity,
            &AngularVelocity,
            &mut ForceAccumulator
        )
    ) {
        let Some(world) = physics_world else { return; };

        world.clear();

        for (entity_id, transform, rigid_body, collider, velocity, angular_velocity, force_accumulator) in bodies {
            let accumulated_force = force_accumulator.0;
            force_accumulator.0 = Vec3::ZERO;

            let body = PhysicsBody::new(
                entity_id.get(),
                rigid_body.clone(),
                collider.clone(),
                transform,
                *velocity,
                *angular_velocity,
                accumulated_force,
            );
            world.add_body(body);
        }

        world.rebuild_broad_phase();
    }
);

system!(
    fn run_physics_step(
        physics_time: res &mut PhysicsTime,
        physics_world: res &mut PhysicsWorld,
    ) {
        let (Some(time), Some(world)) = (physics_time, physics_world) else {
            return;
        };

        let gravity = world.gravity;
        let dt = time.fixed_delta;

        while time.consume_step() {
            for body in world.bodies.iter_mut() {
                if body.rigid_body.is_static() {
                    continue;
                }

                let inverse_mass = body.rigid_body.inverse_mass();
                let external_acceleration = body.accumulated_force * inverse_mass;
                let total_acceleration = gravity + external_acceleration;

                body.velocity.0 += total_acceleration * dt;
                body.position += body.velocity.0 * dt;

                let angular_speed = body.angular_velocity.0.length();
                if angular_speed > f32::EPSILON {
                    let axis = body.angular_velocity.0 / angular_speed;
                    let delta_angle = angular_speed * dt;
                    let delta_rot = Quat::from_axis_angle(axis, delta_angle);
                    body.rotation = (delta_rot * body.rotation).normalize();
                }

                body.accumulated_force = Vec3::ZERO;
            }
        }

        world.rebuild_broad_phase();
    }
);

system!(
    fn sync_physics_to_ecs(
        physics_world: res &PhysicsWorld,
        mut targets: query (
            &EntityId,
            &mut Velocity,
            &mut AngularVelocity,
            &mut Transform
        ),
    ) {
        let Some(world) = physics_world else { return; };

        for (entity_id, velocity, angular_velocity, transform) in targets {
            if let Some(body) = world.get_body(entity_id.get()) {
                velocity.0 = body.velocity.0;
                angular_velocity.0 = body.angular_velocity.0;
                transform.pos = body.position;
                transform.rot = body.rotation;
                transform.scale = body.scale;
            }
        }
    }
);

system!(
    fn emit_physics_events(
        physics_world: res &PhysicsWorld,
        physics_events: res &mut PhysicsEvents
    ) {
        let (Some(world), Some(events)) = (physics_world, physics_events) else {
            return;
        };

        events.contacts.clear();
        events.broad_phase_pairs.clear();
        events.broad_phase_pairs.extend(world.broad_phase_pairs().iter().copied());
    }
);
