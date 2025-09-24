use ecs::*;

#[derive(Component, Default, Debug, PartialEq)]
struct Position(f32);

#[derive(Component, Debug, PartialEq)]
struct Velocity(f32);

#[derive(Resource, Default, Debug, PartialEq)]
struct Counter(u32);

#[test]
fn spawn_entity_assigns_ids_and_entity_component() {
    let mut app = App::new();

    let e0 = app.spawn_entity();
    let e1 = app.spawn_entity();

    assert_eq!(e0, 0);
    assert_eq!(e1, 1);

    let commands: &Commands = &app;
    let world = commands.world;

    let entity_ids = World::get_components::<EntityId>(world);
    let collected: Vec<u32> = entity_ids.into_iter().map(|(_, id)| id.get()).collect();
    assert_eq!(collected, vec![0, 1]);
}

#[test]
fn add_get_and_remove_components_round_trip() {
    let mut app = App::new();
    let entity = app.spawn_entity();

    assert!(app.add_component(entity, Position(3.14)).is_some());
    assert!(app.add_component(entity, Velocity(2.71)).is_some());

    // second insertion should fail without replacing
    assert!(app.add_component(entity, Position(1.0)).is_none());

    let commands: &Commands = &app;
    let world = commands.world;

    let positions = World::get_components::<Position>(world);
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].0, entity);
    assert_eq!(*positions[0].1, Position(3.14));

    let velocities = World::get_components::<Velocity>(world);
    assert_eq!(velocities.len(), 1);
    assert_eq!(velocities[0].0, entity);
    assert_eq!(*velocities[0].1, Velocity(2.71));

    let removed = app.remove_component::<Position>(entity).expect("component missing");
    assert!(removed.as_any().downcast_ref::<Position>().is_some());

    let positions = World::get_components::<Position>(world);
    assert!(positions.is_empty());
}

#[test]
fn insert_resource_overwrites_and_returns_flags() {
    let mut app = App::new();

    assert!(app.insert_resource(Counter(1)).is_some());
    assert!(app.insert_resource(Counter(5)).is_none());

    let commands: &Commands = &app;
    let world = commands.world;

    let counter = World::get_resource::<Counter>(world).expect("resource missing");
    assert_eq!(*counter, Counter(5));
}

system! {
    fn touch_components(query: query (&mut Position, &Velocity)) {
        for (pos, vel) in query {
            pos.0 += vel.0;
        }
    }
}

#[test]
fn systems_modify_components_via_scheduler() {
    let mut app = App::new();
    let entity = app.spawn_entity();
    app.add_component(entity, Position(1.0)).unwrap();
    app.add_component(entity, Velocity(4.0)).unwrap();

    app.add_system(touch_components, SystemStage::Update);
    app.run();

    let commands: &Commands = &app;
    let world = commands.world;

    let positions = World::get_components::<Position>(world);
    assert_eq!(positions[0].1.0, 5.0);
}

#[test]
fn despawn_entity_removes_components() {
    let mut app = App::new();
    let e0 = app.spawn_entity();
    let e1 = app.spawn_entity();

    app.add_component(e0, Position(0.0)).unwrap();
    app.add_component(e1, Position(1.0)).unwrap();

    assert!(app.despawn_entity(e0).is_some());

    let commands: &Commands = &app;
    let world = commands.world;

    let positions = World::get_components::<Position>(world);
    assert_eq!(positions.len(), 1);
    assert_eq!(positions[0].0, e1);
}
