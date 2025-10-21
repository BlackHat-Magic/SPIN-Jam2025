use glam::*;

pub use ecs::*;
pub use networking::*;

pub mod physics;
pub mod render;
pub mod utils;

pub use physics::*;
pub use render::*;
pub use utils::time::*;
pub use utils::*;

#[derive(NetSend, Serialize, Deserialize)]
pub struct TestMessage {
    pub content: String,
}

#[tokio::main]
async fn main() {
    let mut app = App::new();

    let plugins = plugin_group!(
        physics::PhysicsPlugin,
        utils::UtilPlugin::server(),
        networking::NetworkingPlugin::server(),
    );

    app.add_plugin(plugins);

    app.init();

    loop {
        app.run();
        if app.should_exit() {
            break;
        }
    }

    app.de_init();

    std::process::exit(0);
}
