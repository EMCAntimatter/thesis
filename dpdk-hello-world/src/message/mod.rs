use serde::{Deserialize, Serialize};

use crate::log::{LogKey, LogValue};

pub mod message_packet;

pub type MessageCount = u16;
pub type MessageTimestamp = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ClientId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MessageId(pub u32);

impl MessageId {
    pub fn next(&self) -> MessageId {
        let (next_value, _) = self.0.overflowing_add(1);
        MessageId(next_value)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Get {
        key: LogKey
    },
    Set {
        key: LogKey,
        value: LogValue
    }
}