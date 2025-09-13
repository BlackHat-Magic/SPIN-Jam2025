use crate::*;

pub enum SystemRunCriteria {
    Always,
    Once,
    Never,
    OnChannelReceive(String),
}

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

pub struct ResourceAccess {
    pub read: &'static [usize],
    pub write: &'static [usize],
}

pub trait System: Send + Sync + 'static {
    fn name(&self) -> &'static str;
    fn component_access(&self) -> &'static ComponentAccess;
    fn resource_access(&self) -> &'static ResourceAccess;
    fn get_last_run(&self) -> Tick;
    fn set_last_run(&mut self, tick: Tick);

    unsafe fn run_unsafe(&mut self, world: *mut World);
}
