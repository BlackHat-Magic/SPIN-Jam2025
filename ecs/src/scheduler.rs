use crate::*;

pub type Tick = u64;

pub struct Scheduler {
    world: *mut World,
}
