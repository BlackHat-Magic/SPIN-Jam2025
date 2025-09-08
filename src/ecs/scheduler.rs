use crate::ecs::*;

pub struct Scheduler {
    world: *mut World,
}