use ecs::{system, App, Commands, Component, Resource, SystemStage};

#[derive(Component, Default)]
struct Position(u32);

#[derive(Resource, Default)]
struct Counter(u32);

system! {
    fn writer(query: query(&mut Position), counter: res &mut Counter) {
        let Some(counter) = counter else { return; };
        let mut wrote = false;
        for (pos,) in query {
            pos.0 += 1;
            wrote = true;
        }
        if wrote {
            counter.0 += 1;
        }
    }
}

system! {
    fn reader(query: query(&Position), counter: res &mut Counter) {
        let Some(counter) = counter else { return; };
        for (_pos,) in query {
            counter.0 += 10;
        }
    }
}

#[test]
fn scheduler_runs_systems_once_per_tick_in_stage_order() {
    let mut app = App::new();
    let entity = app.spawn_entity();
    app.add_component(entity, Position(0)).unwrap();
    app.insert_resource(Counter::default());

    app.add_system(writer, SystemStage::Update);
    app.add_system(reader, SystemStage::PostUpdate);

    app.run();

    let commands: &Commands = &app;
    let world = commands.world;

    let counter = ecs::World::get_resource::<Counter>(world).unwrap();
    assert_eq!(counter.0, 11); // writer + reader contributions

    let mut positions = ecs::World::get_components::<Position>(world);
    assert_eq!(positions.len(), 1);
    assert_eq!(positions.pop().unwrap().1 .0, 1);
}
