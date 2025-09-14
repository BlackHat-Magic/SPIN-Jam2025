use std::ops::{Deref, DerefMut};

use crate::*;

pub struct App {
    commands: Commands,
}

impl Deref for App {
    type Target = Commands;

    fn deref(&self) -> &Self::Target {
        &self.commands
    }
}

impl DerefMut for App {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.commands
    }
}

impl App {
    pub fn new() -> Self {
        let world = Box::into_raw(Box::new(World::new()));
        let scheduler = Box::into_raw(Box::new(Scheduler::new(world)));
        unsafe {
            (*world).scheduler = scheduler;
        }

        Self {
            commands: Commands::new(world),
        }
    }

    pub fn run(&mut self) {
        unsafe {
            let world = self.commands.world;
            let scheduler = (*world).scheduler;

            Scheduler::run(scheduler, SystemStage::Init);

            loop {
                (*world).tick += 1;

                Scheduler::run(scheduler, SystemStage::PreUpdate);
                Scheduler::run(scheduler, SystemStage::Update);
                Scheduler::run(scheduler, SystemStage::PostUpdate);
                Scheduler::run(scheduler, SystemStage::Render);

                if self.should_exit() {
                    break;
                }
            }
        }
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(self.commands.world);
        }
    }
}

pub struct Commands {
    pub(crate) world: *mut World,
}

pub type EntityId = u32;

impl Commands {
    pub fn new(world: *mut World) -> Self {
        Self { world }
    }

    pub fn spawn_entity(&mut self) -> EntityId {
        unsafe {
            let world = self.world.as_mut().unwrap();
            let id = world.next_entity_id;
            world.next_entity_id += 1;
            let entity = Entity::new(id);
            world.entities.push(entity);
            id
        }
    }

    pub fn despawn_entity(&mut self, id: EntityId) -> Option<()> {
        unsafe {
            let world = self.world.as_mut().unwrap();
            if (id as usize) < world.entities.len() {
                world.entities.remove(id as usize);
                Some(())
            } else {
                None
            }
        }
    }

    pub fn insert_resource<T: Resource>(&mut self, resource: T) -> Option<()> {
        let id = get_resource_id::<T>() as usize;
        unsafe {
            let world = self.world.as_mut().unwrap();
            if world.resources[id].is_none() {
                world.resources[id] = Some(Box::new(resource));
                Some(())
            } else {
                None
            }
        }
    }

    pub fn add_system(&mut self, system: impl System, stage: SystemStage) {
        unsafe {
            let world = self.world.as_mut().unwrap();
            world.systems.push((stage, Box::new(system)));

            // TODO: get it scheduled
        }
    }

    pub fn add_component<T: Component>(&mut self, entity_id: EntityId, component: T) -> Option<()> {
        unsafe {
            let world = self.world.as_mut().unwrap();
            if let Some(entity) = world.entities.get_mut(entity_id as usize) {
                entity.add_component(Box::new(component))
            } else {
                None
            }
        }
    }

    pub fn remove_component<T: Component>(&mut self, entity_id: EntityId) -> Option<Box<dyn Component>> {
        unsafe {
            let world = self.world.as_mut().unwrap();
            if let Some(entity) = world.entities.get_mut(entity_id as usize) {
                entity.remove_component::<T>()
            } else {
                None
            }
        }
    }
    
    pub fn run_system(&mut self, system: &mut dyn System) {
        unsafe {
            system.run_unsafe(self.world);
        }
    }

    pub fn should_exit(&self) -> bool {
        unsafe {
            let world = self.world.as_ref().unwrap();
            world.should_exit
        }
    }

    pub fn exit(&mut self) {
        unsafe {
            let world = self.world.as_mut().unwrap();
            world.should_exit = true;
        }
    }
}

pub struct World {
    pub(crate) entities: Vec<Entity>,
    pub(crate) resources: Vec<Option<Box<dyn Resource>>>,
    pub(crate) systems: Vec<(SystemStage, Box<dyn System>)>,
    pub(crate) tick: Tick,
    pub(crate) next_entity_id: u32,
    pub(crate) scheduler: *mut Scheduler,
    pub(crate) should_exit: bool,
}

impl Drop for World {
    fn drop(&mut self) {
        unsafe {
            if !self.scheduler.is_null() {
                let _ = Box::from_raw(self.scheduler);
            }
        }
    }
}

impl World {
    pub fn new() -> Self {
        let mut resources = Vec::with_capacity(RESOURCE_IDS.get_or_init(crate::build_resource_ids).len());
        resources.resize_with(RESOURCE_IDS.get().unwrap().len(), || None);
        Self {
            entities: Vec::new(),
            resources,
            systems: Vec::new(),
            tick: 0,
            next_entity_id: 0,
            scheduler: std::ptr::null_mut(),
            should_exit: false,
        }
    }

    pub fn get_resource<T: Resource>(world: *mut World) -> Option<&'static T> {
        let id = get_resource_id::<T>() as usize;
        unsafe {
            Some(
                world
                    .as_ref()?
                    .resources
                    .get(id)?
                    .as_ref()?
                    .as_any()
                    .downcast_ref::<T>()
                    .unwrap(),
            )
        }
    }

    pub fn get_resource_mut<T: Resource>(world: *mut World) -> Option<&'static mut T> {
        let id = get_resource_id::<T>() as usize;
        unsafe {
            Some(
                world
                    .as_mut()?
                    .resources
                    .get_mut(id)?
                    .as_mut()?
                    .as_any_mut()
                    .downcast_mut::<T>()
                    .unwrap(),
            )
        }
    }

    pub fn get_components<T: Component>(world: *mut World) -> Vec<(u32, &'static T)> {
        let id = get_component_id::<T>() as usize;
        let mut components = Vec::new();

        unsafe {
            let world = world.as_ref().unwrap();
            for entity in &world.entities {
                if let Some(component) = entity
                    .components
                    .get(id)
                    .and_then(|c| c.as_ref())
                    .and_then(|c| c.as_any().downcast_ref::<T>())
                {
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
                if let Some(component) = entity
                    .components
                    .get_mut(id)
                    .and_then(|c| c.as_mut())
                    .and_then(|c| c.as_any_mut().downcast_mut::<T>())
                {
                    components.push((entity.id, component));
                }
            }
        }

        components
    }
}
