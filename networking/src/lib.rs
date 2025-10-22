use ecs::*;

mod registry;

pub use registry::*;

use anyhow::Result;
use std::any::Any;
use std::collections::VecDeque;
use std::sync::Mutex;

pub use bincode;
pub use net_derive::*;
pub use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::sync::mpsc::*;

pub trait NetSend: Any + Sized + DeserializeOwned {
    fn get_type_id(&self) -> usize;
    fn get_bytes(&self) -> Vec<u8>;
    fn from_bytes(bytes: &[u8]) -> Result<Self>;
}

pub struct NetworkingPlugin {
    is_server: bool,
}

impl NetworkingPlugin {
    pub fn client() -> Self {
        Self { is_server: false }
    }

    pub fn server() -> Self {
        Self { is_server: true }
    }
}

impl Plugin for NetworkingPlugin {
    fn build(&self, app: &mut App) {
        let (tx_event, rx_event) = channel(256);
        let (tx_request, rx_request) = channel(256);

        tokio::spawn(handle_networking(tx_event, rx_request));

        app.insert_resource(Networking::new(tx_request, rx_event));
        app.add_system(gather_events, SystemStage::PreUpdate);
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
        networking.serialize_recv();
    }
}

type RecvChannel = Mutex<VecDeque<(Target, Box<dyn Any>)>>;

#[derive(Resource)]
pub struct Networking {
    tx_request: Sender<NetworkingRequest>,
    rx_event: Receiver<NetworkingEvent>,

    recv_buffer: Vec<RecvChannel>,
    events: Vec<NetworkingEvent>,
}

impl Networking {
    fn new(tx_request: Sender<NetworkingRequest>, rx_event: Receiver<NetworkingEvent>) -> Self {
        let mut recv_buffer = Vec::new();
        let recv_count = registry::NET_IDS.len();
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
            events.push(event);
        }

        self.events = events;
    }

    fn split_off_events(&mut self, cond: fn(&NetworkingEvent) -> bool) -> Vec<NetworkingEvent> {
        let (split_off, events) = self.events.drain(..).partition(cond);

        self.events = events;
        split_off
    }

    fn serialize_recv(&mut self) {
        let split_events = self.split_off_events(|e| matches!(e, NetworkingEvent::RecvData { .. }));

        for event in split_events {
            let NetworkingEvent::RecvData { from, data } = &event else {
                panic!("Event type mismatch in serialize_recv");
            };

            debug_assert!(data.len() > 4);
            let type_id = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;

            debug_assert!(type_id < self.recv_buffer.len());
            let data = &data[4..];

            let Ok(obj) = registry::FROM_BYTES[type_id](data) else {
                println!(
                    "Failed to deserialize network object of type id: {}",
                    type_id,
                );
                continue;
            };
            let mut buffer = self.recv_buffer[type_id].lock().unwrap();
            buffer.push_back((*from, obj));
        }
    }

    pub fn next<T: NetSend>(&self) -> Option<(Target, T)> {
        let type_id = registry::get_net_id::<T>();
        debug_assert!(type_id < self.recv_buffer.len());

        let mut buffer = self.recv_buffer[type_id].lock().unwrap();
        let (target, obj) = buffer.pop_front()?;
        let obj = *obj.downcast::<T>().unwrap();

        Some((target, obj))
    }

    pub fn collect<T: NetSend>(&self) -> Vec<(Target, T)> {
        let type_id = registry::get_net_id::<T>();
        debug_assert!(type_id < self.recv_buffer.len());

        let mut buffer = self.recv_buffer[type_id].lock().unwrap();
        let mut results = Vec::new();

        while let Some((target, obj)) = buffer.pop_front() {
            let obj = *obj.downcast::<T>().unwrap();
            results.push((target, obj));
        }

        results
    }

    pub fn send<T: NetSend>(&self, reliability: Reliability, target: Target, data: T) {
        debug_assert!(target != Target::This, "Cannot send data to 'This' target");

        let mut bytes = Vec::new();
        let type_id = data.get_type_id() as u32;
        bytes.extend_from_slice(&type_id.to_le_bytes());
        bytes.extend_from_slice(&data.get_bytes());

        let request = NetworkingRequest::SendData {
            reliability,
            target,
            data: bytes,
        };

        self.tx_request
            .try_send(request)
            .expect("Networking request buffer full");
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Target {
    All,
    Single(u32),
    This,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NetworkingEvent {
    RecvData { from: Target, data: Vec<u8> },
    Disconnected { target: Target },
    Connected { target: Target },
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Reliability {
    Reliable,
    Unreliable,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum NetworkingRequest {
    Exit,
    SendData {
        reliability: Reliability,
        target: Target,
        data: Vec<u8>,
    },
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
                    NetworkingRequest::SendData { reliability, target, data } => {
                        // Handle sending data over the network
                    }
                }
            }
        }
    }
}
