use glam::Vec3;
use rand::{Rng, SeedableRng, rngs::StdRng};

/// Handle referencing a body stored within a [`PhysicsTestWorld`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BodyHandle(pub(crate) usize);

/// Immutable snapshot of a body's state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BodyState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub mass: f32,
}

/// Parameters describing how to initialise a new body in the harness.
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

/// Helper world for physics tests providing deterministic defaults plus simple integration.
pub struct PhysicsTestWorld {
    gravity: Vec3,
    dt: f32,
    seed: u64,
    rng: StdRng,
    bodies: Vec<TestBody>,
}

impl PhysicsTestWorld {
    /// Construct a world using deterministic defaults (seed = 0, dt = 1/60, gravity = -9.81 m/s²).
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

    /// Change the gravity vector used when stepping bodies.
    pub fn with_gravity(mut self, gravity: Vec3) -> Self {
        self.gravity = gravity;
        self
    }

    /// Change the fixed timestep used for integration.
    pub fn with_dt(mut self, dt: f32) -> Self {
        self.dt = dt;
        self
    }

    /// Override the RNG seed, allowing deterministic randomised fixtures.
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.reseed(seed);
        self
    }

    /// Reset the underlying random number generator to a specific seed.
    pub fn reseed(&mut self, seed: u64) {
        self.seed = seed;
        self.rng = StdRng::seed_from_u64(seed);
    }

    /// Returns the gravity vector used by this world.
    pub fn gravity(&self) -> Vec3 {
        self.gravity
    }

    /// Returns the fixed timestep used by this world.
    pub fn dt(&self) -> f32 {
        self.dt
    }

    /// Spawn a new body with the provided initial conditions.
    pub fn add_body(&mut self, init: BodyInit) -> BodyHandle {
        let handle = BodyHandle(self.bodies.len());
        self.bodies.push(TestBody::new(init));
        handle
    }

    /// Spawn a body with pseudo-random initial conditions derived from an internal RNG.
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

    /// Returns the number of active bodies.
    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    /// Fetch a copy of the current body state, if the handle is valid.
    pub fn body_state(&self, handle: BodyHandle) -> Option<BodyState> {
        self.bodies.get(handle.0).map(TestBody::state)
    }

    /// Advance the simple integrator by `steps` ticks, mutating all bodies in-place.
    pub fn step(&mut self, steps: u32) {
        for _ in 0..steps {
            for body in &mut self.bodies {
                body.velocity += self.gravity * self.dt;
                body.position += body.velocity * self.dt;
            }
        }
    }

    /// Compute the total kinetic energy of the system (Σ 1/2 * m * |v|²).
    pub fn total_kinetic_energy(&self) -> f32 {
        self.bodies
            .iter()
            .map(|body| 0.5 * body.mass * body.velocity.length_squared())
            .sum()
    }

    /// Compute the total gravitational potential energy relative to the origin.
    pub fn total_potential_energy(&self) -> f32 {
        let g = self.gravity;
        self.bodies
            .iter()
            .map(|body| -body.mass * g.dot(body.position))
            .sum()
    }

    /// Convenience helper returning the sum of kinetic and potential energy.
    pub fn total_energy(&self) -> f32 {
        self.total_kinetic_energy() + self.total_potential_energy()
    }

    /// Remove all bodies from the world.
    pub fn clear_bodies(&mut self) {
        self.bodies.clear();
    }
}
