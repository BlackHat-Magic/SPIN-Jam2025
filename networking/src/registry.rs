pub use typeid::ConstTypeId;

use crate::*;
use lazy_static::lazy_static;
use std::collections::HashMap;

pub struct NetSendRegistration {
    pub type_id: ConstTypeId,
    pub name: &'static str,
}

pub struct NetRecvRegistration {
    pub type_id: ConstTypeId,
    pub name: &'static str,
    pub from_bytes: fn(&[u8]) -> Box<dyn Any>,
}

pub fn from_bytes<T: NetRecv>(bytes: &[u8]) -> Box<T> {
    let index = get_recv_id::<T>();
    (FROM_BYTES[index])(bytes)
        .downcast::<T>()
        .expect("Failed to downcast Box<dyn Any> to Box<T>. Indicates mismatched type IDs and constructors.")
}

pub fn get_send_id<T: NetSend>() -> usize {
    *SEND_IDS.get(&ConstTypeId::of::<T>()).expect(
        "Type not registered as NetSend. You must use the Derive macro to register the type.",
    )
}

pub fn get_recv_id<T: NetRecv>() -> usize {
    *RECV_IDS.get(&ConstTypeId::of::<T>()).expect(
        "Type not registered as NetRecv. You must use the Derive macro to register the type.",
    )
}

inventory::collect!(NetRecvRegistration);
inventory::collect!(NetSendRegistration);

type FromBytes = fn(&[u8]) -> Box<dyn Any>;

lazy_static! {
    pub static ref SEND_IDS: HashMap<SendId, usize> = build_send_ids();
    pub static ref RECV_IDS: HashMap<RecvId, usize> = build_recv_ids();
    pub static ref FROM_BYTES: Vec<FromBytes> = {
        let mut entries: Vec<_> = inventory::iter::<NetRecvRegistration>.into_iter().collect();
        entries.sort_by_key(|e| RECV_IDS[&e.type_id]);
        entries.into_iter().map(|r| r.from_bytes).collect()
    };
}

pub type SendId = ConstTypeId;
pub type RecvId = ConstTypeId;

fn build_send_ids() -> HashMap<SendId, usize> {
    let mut entries: Vec<_> = inventory::iter::<NetSendRegistration>.into_iter().collect();
    entries.sort_by_key(|e| e.name);
    entries
        .into_iter()
        .enumerate()
        .map(|(i, r)| (r.type_id, i))
        .collect()
}

fn build_recv_ids() -> HashMap<RecvId, usize> {
    let mut entries: Vec<_> = inventory::iter::<NetRecvRegistration>.into_iter().collect();
    entries.sort_by_key(|e| e.name);
    entries
        .into_iter()
        .enumerate()
        .map(|(i, r)| (r.type_id, i))
        .collect()
}
