pub mod query;
pub mod scheduler;
pub mod system;
pub mod world;

use typeid::ConstTypeId;

use std::any::Any;
use std::collections::HashMap;
use std::sync::OnceLock;

pub use derive::Component;
pub use derive::Resource;

pub use query::{Query, QueryPattern};
pub use scheduler::Scheduler;
pub use system::System;
pub use world::World;

pub trait Component: Any {
    fn get_type_id(&self) -> usize;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub trait Resource: Any {
    fn get_type_id(&self) -> usize;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct ComponentRegistration {
    pub type_id: ConstTypeId,
    pub name: &'static str,
}

pub struct ResourceRegistration {
    pub type_id: ConstTypeId,
    pub name: &'static str,
}

inventory::collect!(ComponentRegistration);
inventory::collect!(ResourceRegistration);

static COMPONENT_IDS: OnceLock<HashMap<ConstTypeId, usize>> = OnceLock::new();
static RESOURCE_IDS: OnceLock<HashMap<ConstTypeId, usize>> = OnceLock::new();

fn build_component_ids() -> HashMap<ConstTypeId, usize> {
    let mut entries: Vec<_> = inventory::iter::<ComponentRegistration>
        .into_iter()
        .collect();
    entries.sort_by_key(|e| e.name);
    entries
        .into_iter()
        .enumerate()
        .map(|(i, r)| (r.type_id, i))
        .collect()
}

fn build_resource_ids() -> HashMap<ConstTypeId, usize> {
    let mut entries: Vec<_> = inventory::iter::<ResourceRegistration>
        .into_iter()
        .collect();
    entries.sort_by_key(|e| e.name);
    entries
        .into_iter()
        .enumerate()
        .map(|(i, r)| (r.type_id, i))
        .collect()
}

pub fn get_component_id<T: 'static>() -> usize {
    *COMPONENT_IDS
        .get_or_init(build_component_ids)
        .get(&ConstTypeId::of::<T>())
        .expect("Component not registered")
}

pub fn get_resource_id<T: 'static>() -> usize {
    *RESOURCE_IDS
        .get_or_init(build_resource_ids)
        .get(&ConstTypeId::of::<T>())
        .expect("Resource not registered")
}

pub struct Entity {
    pub id: u32,
    pub(crate) components: Vec<Option<Box<dyn Component>>>,
}

impl Entity {
    pub fn new(id: u32) -> Self {
        Self {
            id,
            components: Vec::with_capacity(COMPONENT_IDS.get().unwrap().len()),
        }
    }

    pub fn set_component(&mut self, component: Option<Box<dyn Component>>, id: usize) {
        self.components[id] = component;
    }

    pub fn add_component(&mut self, component: Box<dyn Component>) -> Option<()> {
        let id = component.get_type_id();

        if self.components[id].is_none() {
            self.components[id] = Some(component);
            Some(())
        } else {
            None
        }
    }

    pub fn get_component<T: Component>(&self) -> Option<&T> {
        let id = get_component_id::<T>() as usize;
        self.components[id]
            .as_ref()
            .and_then(|c| c.as_any().downcast_ref::<T>())
    }

    pub fn get_component_mut<T: Component>(&mut self) -> Option<&mut T> {
        let id = get_component_id::<T>() as usize;
        self.components[id]
            .as_mut()
            .and_then(|c| c.as_any_mut().downcast_mut::<T>())
    }

    pub fn remove_component<T: Component>(&mut self) -> Option<Box<dyn Component>> {
        let id = get_component_id::<T>() as usize;
        self.components[id].take()
    }

    pub fn has_component<T: Component>(&self) -> bool {
        let id = get_component_id::<T>() as usize;
        self.components[id].is_some()
    }
}
