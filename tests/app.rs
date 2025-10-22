use rust_game_engine::*;

#[derive(Resource, Default)]
struct StageLog(Vec<&'static str>);

system! {
    fn log_pre(log: res &mut StageLog) {
        if let Some(log) = log {
            log.0.push("pre");
        }
    }
}

system! {
    fn log_update(log: res &mut StageLog) {
        if let Some(log) = log {
            log.0.push("update");
        }
    }
}

system! {
    fn log_post(log: res &mut StageLog) {
        if let Some(log) = log {
            log.0.push("post");
        }
    }
}

system! {
    fn log_render(log: res &mut StageLog) {
        if let Some(log) = log {
            log.0.push("render");
        }
    }
}

#[test]
fn systems_execute_in_defined_stage_order() {
    let mut app = App::new();

    app.insert_resource(StageLog::default());

    app.add_system(log_pre, SystemStage::PreUpdate);
    app.add_system(log_update, SystemStage::Update);
    app.add_system(log_post, SystemStage::PostUpdate);
    app.add_system(log_render, SystemStage::Render);

    app.run();

    let commands: &Commands = &app;
    let world_ptr = commands.world;
    let log =
        unsafe { World::get_resource::<StageLog>(world_ptr).expect("StageLog resource not found") };
    assert_eq!(log.0, vec!["pre", "update", "post", "render"]);
}
