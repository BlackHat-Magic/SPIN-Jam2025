use glam::Vec3;

/// Helper world for physics tests providing deterministic defaults.
pub struct PhysicsTestWorld {
    gravity: Vec3,
    dt: f32,
    body_count: usize,
}

impl PhysicsTestWorld {
    pub fn new() -> Self {
        Self {
            gravity: Vec3::new(0.0, -9.81, 0.0),
            dt: 1.0 / 60.0,
            body_count: 0,
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

    pub fn gravity(&self) -> Vec3 {
        self.gravity
    }

    pub fn dt(&self) -> f32 {
        self.dt
    }

    pub fn body_count(&self) -> usize {
        self.body_count
    }
}
