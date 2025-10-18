use std::ops::Deref;

use crate::*;
use rayon::prelude::*;

pub type Tick = u64;

pub struct Scheduler {
    world: *mut World,

    systems: HashMap<SystemStage, Vec<Vec<*mut dyn System>>>,
}

#[derive(Clone, Copy)]
struct WorldWrapper(*mut World);

unsafe impl Send for WorldWrapper {}
unsafe impl Sync for WorldWrapper {}

impl Deref for WorldWrapper {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

#[derive(Clone, Copy)]
struct SystemWrapper(*mut dyn System);

unsafe impl Send for SystemWrapper {}
unsafe impl Sync for SystemWrapper {}

impl Scheduler {
    pub(crate) fn new(world: *mut World) -> Self {
        Self {
            world,
            systems: HashMap::new(),
        }
    }

    pub(crate) fn run(scheduler: *mut Scheduler, stage: SystemStage) {
        unsafe {
            let world = (*scheduler).world;
            let Some(systems) = (*scheduler).systems.get(&stage) else {
                return;
            };
            for group in systems {
                if group.len() == 1 {
                    // Run single systems on main thread because they might not be Send + Sync
                    let system = group[0];
                    system.as_mut().unwrap().run_unsafe(world);
                    continue;
                }

                let group: Vec<SystemWrapper> = group.iter().map(|&s| SystemWrapper(s)).collect();
                let world = WorldWrapper(world);

                group.par_iter().for_each(|system| {
                    let world = world;
                    let world = world.0;

                    let system = system.0;

                    system.as_mut().unwrap().run_unsafe(world);
                });
            }
        }
    }

    pub(crate) fn add_system(&mut self, system: *mut dyn System, stage: SystemStage) {
        let entry = self.systems.entry(stage).or_insert_with(Vec::new);
        if unsafe { system.as_ref() }.unwrap().runs_alone() || entry.is_empty() {
            entry.push(vec![system]);
            return;
        }

        for group in entry.iter_mut() {
            if unsafe { group[0].as_ref() }.unwrap().runs_alone() {
                continue;
            }
            let mut overlap = false;
            for &existing_system in group.iter() {
                let existing_component_access = unsafe { (*existing_system).component_access() };
                let new_component_access = unsafe { (*system).component_access() };
                if existing_component_access.overlaps(&new_component_access) {
                    overlap = true;
                    break;
                }

                let existing_resource_access = unsafe { (*existing_system).resource_access() };
                let new_resource_access = unsafe { (*system).resource_access() };
                if existing_resource_access.overlaps(&new_resource_access) {
                    overlap = true;
                    break;
                }
            }
            if !overlap {
                group.push(system);
                return;
            }
        }

        entry.push(vec![system]);
    }
}
