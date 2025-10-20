use ecs::*;

mod registry;

pub use registry::*;

use std::any::Any;
use std::collections::VecDeque;
use std::sync::Mutex;

pub use bincode;
pub use net_derive::*;
pub use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::sync::mpsc::*;

pub trait NetSend: Serialize {
    fn get_type_id(&self) -> usize;
    fn get_bytes(&self) -> Vec<u8>;
}

pub trait NetRecv: Any + Sized + DeserializeOwned {
    fn get_type_id(&self) -> usize;
    fn from_bytes(bytes: &[u8]) -> Self;
}

pub struct NetworkingPlugin;

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        let (tx_event, rx_event) = channel(32);
        let (tx_request, rx_request) = channel(32);

        tokio::spawn(handle_networking(tx_event, rx_request));

        app.insert_resource(Networking::new(tx_request, rx_event));
        app.add_system(gather_events, SystemStage::PreUpdate);
        app.add_system(serialize_recv, SystemStage::PreUpdate);
    }
}

system! {
    fn gather_events(
        networking: res &mut Networking,
    ) {
        let Some(networking) = networking else {
            return;
        };

        networking.gather_recv();
    }
}

system! {
    fn serialize_recv(
        networking: res &mut Networking,
    ) {
        let Some(networking) = networking else {
            return;
        };

        networking.serialize_recv();
    }
}

#[derive(Resource)]
pub struct Networking {
    tx_request: Sender<NetworkingRequest>,
    rx_event: Receiver<NetworkingEvent>,

    recv_buffer: Vec<Mutex<VecDeque<Box<dyn Any>>>>,
    events: Vec<NetworkingEvent>,
}

impl Networking {
    fn new(tx_request: Sender<NetworkingRequest>, rx_event: Receiver<NetworkingEvent>) -> Self {
        let mut recv_buffer = Vec::new();
        let recv_count = registry::RECV_IDS.len();
        for _ in 0..recv_count {
            recv_buffer.push(Mutex::new(VecDeque::new()));
        }

        Self {
            tx_request,
            rx_event,
            recv_buffer,
            events: Vec::new(),
        }
    }

    fn gather_recv(&mut self) {
        let mut events = Vec::new();

        while let Ok(event) = self.rx_event.try_recv() {
            match &event {
                NetworkingEvent::RecvData { from: _, data } => {
                    debug_assert!(data.len() > 4);
                    let type_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

                    debug_assert!(type_id < self.recv_buffer.len());
                    let data = &data[4..];
                }
                _ => {}
            }

            events.push(event);
        }

        self.events = events;
    }

    fn split_off_events(&mut self, cond: fn(&NetworkingEvent) -> bool) -> Vec<NetworkingEvent> {
        let (split_off, events) = self.events.drain(..).partition(|e| cond(e));

        self.events = events;
        split_off
    }

    fn serialize_recv(&mut self) {
        let split_events = self.split_off_events(|e| matches!(e, NetworkingEvent::RecvData { .. }));

        for event in split_events {
            match event {
                NetworkingEvent::RecvData { from: _, data } => {
                    debug_assert!(data.len() > 4);
                    let type_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

                    debug_assert!(type_id < self.recv_buffer.len());
                    let data = &data[4..];

                    let obj = registry::FROM_BYTES[type_id](data);
                    let mut buffer = self.recv_buffer[type_id].lock().unwrap();
                    buffer.push_back(obj);
                }
                _ => panic!("split off split off wrong????"),
            }
        }
    }
}

pub enum Target {
    All,
    Single(u32),
}

pub enum NetworkingEvent {
    RecvData { from: Target, data: Vec<u8> },
}

pub enum NetworkingRequest {
    Exit,
    SendData { target: Target, data: Vec<u8> },
}

async fn handle_networking(
    mut tx_event: Sender<NetworkingEvent>,
    mut rx_request: Receiver<NetworkingRequest>,
) {
    loop {
        tokio::select! {
            request = rx_request.recv() => {
                let Some(request) = request else {
                    break;
                };

                match request {
                    NetworkingRequest::Exit => break,
                    NetworkingRequest::SendData { target, data } => {
                        // Handle sending data over the network
                    }
                }
            }
        }
    }
}
