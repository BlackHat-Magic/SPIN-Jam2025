use ecs::*;

use std::sync::mpsc::*;

pub struct NetworkingPlugin;

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        let (tx_event, rx_event) = channel();
        let (tx_request, rx_request) = channel();

        std::thread::spawn(move || {
            handle_networking(tx_event, rx_request);
        });

        app.insert_resource(Networking {
            tx_request,
            rx_event,
        });
    }
}

#[derive(Resource)]
pub struct Networking {
    tx_request: Sender<NetworkingRequest>,
    rx_event: Receiver<NetworkingEvent>,
}

pub enum NetworkingEvent {}

pub enum NetworkingRequest {}

fn handle_networking(tx_event: Sender<NetworkingEvent>, rx_request: Receiver<NetworkingRequest>) {}
