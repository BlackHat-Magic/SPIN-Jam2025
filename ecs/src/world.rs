use crate::*;

pub struct World {
    entities: Vec<Entity>,
    resources: Vec<Option<Box<dyn Resource>>>,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: Vec::new(),
            resources: Vec::with_capacity(RESOURCE_IDS.get().unwrap().len()),
        }
    }

    pub fn get_resource<T: Resource>(world: *mut World) -> Option<&'static T> {
        let id = get_resource_id::<T>() as usize;
        unsafe { Some(world.as_ref()?.resources.get(id)?.as_ref()?.as_any().downcast_ref::<T>().unwrap()) }
    }

    pub fn get_resource_mut<T: Resource>(world: *mut World) -> Option<&'static mut T> {
        let id = get_resource_id::<T>() as usize;
        unsafe { Some(world.as_mut()?.resources.get_mut(id)?.as_mut()?.as_any_mut().downcast_mut::<T>().unwrap()) }
    }

    pub fn get_components<T: Component>(world: *mut World) -> Vec<(u32, &'static T)> {
        let id = get_component_id::<T>() as usize;
        let mut components = Vec::new();

        unsafe {
            let world = world.as_ref().unwrap();
            for entity in &world.entities {
                if let Some(component) = entity.components.get(id).and_then(|c| c.as_ref()).and_then(|c| c.as_any().downcast_ref::<T>()) {
                    components.push((entity.id, component));
                }
            }
        }

        components
    }

    pub fn get_components_mut<T: Component>(world: *mut World) -> Vec<(u32, &'static mut T)> {
        let id = get_component_id::<T>() as usize;
        let mut components = Vec::new();

        unsafe {
            let world = world.as_mut().unwrap();
            for entity in &mut world.entities {
                if let Some(component) = entity.components.get_mut(id).and_then(|c| c.as_mut()).and_then(|c| c.as_any_mut().downcast_mut::<T>()) {
                    components.push((entity.id, component));
                }
            }
        }

        components
    }
}
