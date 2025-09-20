#![allow(incomplete_features)]
#![feature(specialization)]

pub mod scheduler;
pub mod system;
pub mod world;

use std::any::Any;
use std::collections::HashMap;
use std::sync::OnceLock;

pub use inventory::submit;
pub use typeid::ConstTypeId;

pub use derive::*;

pub use scheduler::*;
pub use system::*;
pub use world::*;

pub use lazy_static::lazy_static;

pub trait SendSyncCheck {
    fn is_not_send_sync() -> bool;
}

impl<T: Send + Sync + Any> SendSyncCheck for T {
    fn is_not_send_sync() -> bool {
        false
    }
}

impl<T: Any> SendSyncCheck for T {
    default fn is_not_send_sync() -> bool {
        true
    }
}

pub trait Plugin {
    fn build(&self, app: &mut App);
}

pub struct PluginGroup {
    plugins: Vec<Box<dyn Plugin>>,
}

impl Plugin for PluginGroup {
    fn build(&self, app: &mut App) {
        for plugin in &self.plugins {
            plugin.build(app);
        }
    }
}

impl PluginGroup {
    pub fn new() -> Self {
        Self {
            plugins: vec![]
        }
    }

    pub fn add(&mut self, plugin: Box<dyn Plugin>) {
        self.plugins.push(plugin);
    }
}

#[macro_export]
macro_rules! plugin_group {
    ($($plugin:expr),* $(,)?) => {
        {
            let mut group = PluginGroup::new();
            $(
                group.add(Box::new($plugin));
            )*
            group
        }
    };
}

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

static COMPONENT_IDS: OnceLock<HashMap<ComponentId, usize>> = OnceLock::new();
static RESOURCE_IDS: OnceLock<HashMap<ResourceId, usize>> = OnceLock::new();

pub type ComponentId = ConstTypeId;
pub type ResourceId = ConstTypeId;

fn build_component_ids() -> HashMap<ComponentId, usize> {
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

fn build_resource_ids() -> HashMap<ResourceId, usize> {
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

pub fn get_component_id<T>() -> usize {
    *COMPONENT_IDS
        .get_or_init(build_component_ids)
        .get(&ConstTypeId::of::<T>())
        .expect("Component not registered")
}

pub fn get_resource_id<T>() -> usize {
    *RESOURCE_IDS
        .get_or_init(build_resource_ids)
        .get(&ConstTypeId::of::<T>())
        .expect("Resource not registered")
}

#[derive(Component)]
pub struct EntityId {
    id: u32,
}

impl EntityId {
    pub fn get(&self) -> u32 {
        self.id
    }
}

pub struct Entity {
    pub id: u32,
    pub(crate) components: Vec<Option<Box<dyn Component>>>,
}

impl Entity {
    pub fn new(id: u32) -> Self {
        let mut components =
            Vec::with_capacity(COMPONENT_IDS.get_or_init(build_component_ids).len());
        components.resize_with(COMPONENT_IDS.get_or_init(build_component_ids).len(), || {
            None
        });
        let mut result = Self { id, components };
        result.add_component(Box::new(EntityId { id })).unwrap();
        result
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
