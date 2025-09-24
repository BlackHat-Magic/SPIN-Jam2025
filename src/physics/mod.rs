use crate::*;
use glam::{Mat4, Quat, Vec3};

pub mod test;

pub use test::PhysicsTestWorld;

pub struct PhysicsPlugin;

impl Plugin for PhysicsPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(Component)]
pub struct Transform {
    pub pos: Vec3,
    pub scale: Vec3,
    pub rot: Quat,
}

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
