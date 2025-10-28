use crate::*;
use glam::{Vec3};


pub struct Wall {
    pub p1: Vec3,
    pub p2: Vec3,
}
#[derive(Component)]
pub struct Walls (pub Vec<Wall>);