use crate::*;

pub enum SystemRunCriteria {
    Always,
    Once,
    Never,
    OnChannelReceive(String),
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub enum SystemStage {
    Init,
    PreUpdate,
    Update,
    PostUpdate,
    Render,
}

pub struct ComponentAccess {
    pub read: &'static [usize],
    pub write: &'static [usize],
}

impl ComponentAccess {
    pub fn overlaps(&self, other: &ComponentAccess) -> bool {
        for &r in self.read {
            if other.write.contains(&r) {
                return true;
            }
        }

        for &w in self.write {
            if other.read.contains(&w) || other.write.contains(&w) {
                return true;
            }
        }

        false
    }
}

pub struct ResourceAccess {
    pub read: &'static [usize],
    pub write: &'static [usize],
}

impl ResourceAccess {
    pub fn overlaps(&self, other: &ResourceAccess) -> bool {
        for &r in self.read {
            if other.write.contains(&r) {
                return true;
            }
        }

        for &w in self.write {
            if other.read.contains(&w) || other.write.contains(&w) {
                return true;
            }
        }

        false
    }
}

pub trait System: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn component_access(&self) -> &'static ComponentAccess;
    fn resource_access(&self) -> &'static ResourceAccess;
    fn get_last_run(&self) -> Tick;
    fn set_last_run(&mut self, tick: Tick);
    fn runs_alone(&self) -> bool;

    unsafe fn run_unsafe(&mut self, world: *mut World);
}
