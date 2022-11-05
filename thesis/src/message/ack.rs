use serde::{Deserialize, Serialize};

use super::client_message::{ClientId, MessageId};

#[repr(C)]
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AckMessage<LogValueType> {
    pub client_id: ClientId,
    pub message_id: MessageId,
    pub extension: AckMessageExtensions<LogValueType>,
}

impl<LogValueType> Default for AckMessage<LogValueType> {
    fn default() -> Self {
        Self {
            client_id: super::client_message::ClientId(0),
            message_id: super::client_message::MessageId(0),
            extension: AckMessageExtensions::None,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AckMessageExtensions<LogValueType> {
    None,
    Get(Option<LogValueType>),
    Put(Option<LogValueType>),
    Delete(Option<LogValueType>),
}
