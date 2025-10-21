pub use typeid::ConstTypeId;

use anyhow::Result;

use crate::*;
use lazy_static::lazy_static;
use std::collections::HashMap;

pub struct NetRegistration {
    pub type_id: ConstTypeId,
    pub name: &'static str,
    pub from_bytes: fn(&[u8]) -> Result<Box<dyn Any>>,
}

pub fn from_bytes<T: NetSend>(bytes: &[u8]) -> Result<Box<T>> {
    let index = get_net_id::<T>();
    FROM_BYTES[index](bytes)?
        .downcast::<T>()
        .map_err(|_| anyhow::anyhow!("Failed to downcast Box<dyn Any> to Box<T>"))
}

pub fn get_net_id<T: NetSend>() -> usize {
    *NET_IDS.get(&ConstTypeId::of::<T>()).expect(
        "Type not registered as NetRecv. You must use the Derive macro to register the type.",
    )
}

inventory::collect!(NetRegistration);

type FromBytes = fn(&[u8]) -> Result<Box<dyn Any>>;

lazy_static! {
    pub static ref NET_IDS: HashMap<NetId, usize> = build_net_ids();
    pub static ref FROM_BYTES: Vec<FromBytes> = {
        let mut entries: Vec<_> = inventory::iter::<NetRegistration>.into_iter().collect();
        entries.sort_by_key(|e| NET_IDS[&e.type_id]);
        entries.into_iter().map(|r| r.from_bytes).collect()
    };
}

pub type NetId = ConstTypeId;

fn build_net_ids() -> HashMap<NetId, usize> {
    let mut entries: Vec<_> = inventory::iter::<NetRegistration>.into_iter().collect();
    entries.sort_by_key(|e| e.name); // ensures deterministic ordering
    entries
        .into_iter()
        .enumerate()
        .map(|(i, r)| (r.type_id, i))
        .collect()
}
