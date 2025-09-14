use crate::*;

pub type Tick = u64;

pub struct Scheduler {
    world: *mut World,
}

impl Scheduler {
    pub fn new(world: *mut World) -> Self {
        Self { world }
    }

    pub fn run(scheduler: *mut Scheduler, stage: SystemStage) {
        unsafe {
            let world = (*scheduler).world;
            let systems = &mut (*world).systems;

            for (system_stage, system) in systems.iter_mut() {
                if *system_stage == stage {
                    system.run_unsafe(world);
                }
            }
        }
    }
}
