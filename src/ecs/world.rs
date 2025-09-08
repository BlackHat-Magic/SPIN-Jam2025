use super::{Entity, Resource, system::System};

pub struct World {
    entities: Vec<Entity>,
    resources: Vec<Option<Box<dyn Resource>>>,
    systems: Vec<Box<dyn System>>,
}