use std::thread;
use std::time::Duration;

use klaus_of_death::utils::time::{self, Time};
use klaus_of_death::{App, Commands, SystemStage, World};

#[test]
fn time_systems_initialize_and_update_delta() {
    let mut app = App::new();

    app.add_system(time::init_time, SystemStage::Init);
    app.add_system(time::update_time, SystemStage::PreUpdate);

    app.init();

    let commands: &Commands = &app;
    let world_ptr = commands.world;

    let time = World::get_resource::<Time>(world_ptr).expect("Time resource not initialized");
    assert_eq!(time.delta_seconds, 0.0);

    thread::sleep(Duration::from_millis(5));

    app.run();

    let time = World::get_resource::<Time>(world_ptr).expect("Time resource not present");
    assert!(time.delta_seconds > 0.0);
}
