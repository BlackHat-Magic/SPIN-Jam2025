use crate::*;

pub trait System: Send + Sync {
    // should only be called by the scheduler
    unsafe fn run(&mut self, world: *mut World);
}

pub struct SystemRegistration {
    pub name: &'static str,
    pub constructor: fn() -> Box<dyn System>,
}

inventory::collect!(SystemRegistration);
