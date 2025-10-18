use ecs::*;

#[derive(Component, Default, Debug, PartialEq)]
struct Foo(u32);

#[test]
fn component_macro_registers_type_and_allows_access() {
    let mut app = App::new();
    let entity = app.spawn_entity();
    app.add_component(entity, Foo(42)).unwrap();

    let commands: &Commands = &app;
    let world_ptr = commands.world;
    let components = unsafe { World::get_components::<Foo>(world_ptr) };

    assert_eq!(components.len(), 1);
    assert_eq!(components[0].0, entity);
    assert_eq!(*components[0].1, Foo(42));
}
