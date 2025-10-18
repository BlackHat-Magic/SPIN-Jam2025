use glam::Vec3;
use rand::{Rng, SeedableRng, rngs::StdRng};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BodyHandle(pub(crate) usize);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub mass: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyInit {
    pub position: Vec3,
    pub velocity: Vec3,
    pub mass: f32,
}

impl Default for BodyInit {
    fn default() -> Self {
        Self {
            position: Vec3::ZERO,
            velocity: Vec3::ZERO,
            mass: 1.0,
        }
    }
}

#[derive(Clone, Debug)]
struct TestBody {
    position: Vec3,
    velocity: Vec3,
    mass: f32,
}

impl TestBody {
    fn new(init: BodyInit) -> Self {
        Self {
            position: init.position,
            velocity: init.velocity,
            mass: init.mass.max(f32::EPSILON),
        }
    }

    fn state(&self) -> BodyState {
        BodyState {
            position: self.position,
            velocity: self.velocity,
            mass: self.mass,
        }
    }
}

pub struct PhysicsTestWorld {
    gravity: Vec3,
    dt: f32,
    seed: u64,
    rng: StdRng,
    bodies: Vec<TestBody>,
}

impl PhysicsTestWorld {
    pub fn new() -> Self {
        let seed = 0;
        Self {
            gravity: Vec3::new(0.0, -9.81, 0.0),
            dt: 1.0 / 60.0,
            seed,
            rng: StdRng::seed_from_u64(seed),
            bodies: Vec::new(),
        }
    }

    pub fn with_gravity(mut self, gravity: Vec3) -> Self {
        self.gravity = gravity;
        self
    }

    pub fn with_dt(mut self, dt: f32) -> Self {
        self.dt = dt;
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.reseed(seed);
        self
    }

    pub fn reseed(&mut self, seed: u64) {
        self.seed = seed;
        self.rng = StdRng::seed_from_u64(seed);
    }

    pub fn gravity(&self) -> Vec3 {
        self.gravity
    }

    pub fn dt(&self) -> f32 {
        self.dt
    }

    pub fn add_body(&mut self, init: BodyInit) -> BodyHandle {
        let handle = BodyHandle(self.bodies.len());
        self.bodies.push(TestBody::new(init));
        handle
    }

    pub fn spawn_random_body(&mut self) -> BodyHandle {
        let rng = &mut self.rng;
        let position = Vec3::new(
            rng.gen_range(-2.0..=2.0),
            rng.gen_range(0.5..=3.0),
            rng.gen_range(-2.0..=2.0),
        );
        let velocity = Vec3::new(
            rng.gen_range(-1.0..=1.0),
            rng.gen_range(-1.0..=1.0),
            rng.gen_range(-1.0..=1.0),
        );

        self.add_body(BodyInit {
            position,
            velocity,
            mass: 1.0,
        })
    }

    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    pub fn body_state(&self, handle: BodyHandle) -> Option<BodyState> {
        self.bodies.get(handle.0).map(TestBody::state)
    }

    pub fn step(&mut self, steps: u32) {
        for _ in 0..steps {
            for body in &mut self.bodies {
                body.velocity += self.gravity * self.dt;
                body.position += body.velocity * self.dt;
            }
        }
    }

    pub fn total_kinetic_energy(&self) -> f32 {
        self.bodies
            .iter()
            .map(|body| 0.5 * body.mass * body.velocity.length_squared())
            .sum()
    }

    pub fn total_potential_energy(&self) -> f32 {
        let g = self.gravity;
        self.bodies
            .iter()
            .map(|body| -body.mass * g.dot(body.position))
            .sum()
    }

    pub fn total_energy(&self) -> f32 {
        self.total_kinetic_energy() + self.total_potential_energy()
    }

    pub fn clear_bodies(&mut self) {
        self.bodies.clear();
    }
}
