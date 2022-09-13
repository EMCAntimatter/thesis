pub mod slab_list;
pub mod client_log_manager;

use crate::message::{MessageTimestamp, MessageId, ClientId, ClientMessage};

pub type LogKey = u32;
pub type LogValue = u32;

#[repr(C)]
#[derive(Clone)]
pub struct LogMessage {
    timestamp: MessageTimestamp,
    client_id: ClientId,
    message_id: MessageId,
    message: ClientMessage
}

impl LogMessage {
    #[inline]
    pub fn tag(&self) -> u128 {
        // unsafe version
        *unsafe { std::mem::transmute::<&Self, &u128>(self) }

        // // safe version
        // ((self.timestamp as u128) << 64) | ((self.client_id.0 as u128) << 32) | (self.message_id.0 as u128)
    }
}

impl PartialEq for LogMessage {
    fn eq(&self, other: &Self) -> bool {        
        self.tag() == other.tag()
    }
}

impl Eq for LogMessage {}

impl PartialOrd for LogMessage {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for LogMessage {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.tag().cmp(&other.tag())
    }
}
