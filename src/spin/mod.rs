use crate::*;
use glam::Vec3;

pub mod post;

#[derive(Resource)]
pub struct PlayerPosition(pub Vec3);

pub struct Wall {
    pub p1: Vec3,
    pub p2: Vec3,
}
#[derive(Component)]
pub struct Walls(pub Vec<Wall>);

pub enum AIState {
    Idle,
    Sus(f32),
    Noticed(f32),
    Chase(bool),
    Search(f32),
}
#[derive(Component)]
pub struct Ai {
    pub last_position: Vec3,
    pub state: AIState,
}
