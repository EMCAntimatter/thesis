pub mod dev;
pub mod eth;
#[allow(clippy::module_inception)]
pub mod event;
pub mod event_interface;

pub type EventDeviceId = u8;
pub type EventPortId = u8;
