use serde::{Deserialize, Serialize};

use self::{ack::AckMessage, client_message::ClientLogMessage};

pub mod ack;
pub mod client_message;

/// All messages in the program that can be received by the DPDK-bound ports
#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Message<LogKeyType, LogValueType> {
    IncomingClientMessage(ClientLogMessage<LogKeyType, LogValueType>),
    AckMessage(AckMessage<LogValueType>),
}

impl<LogKeyType, LogValueType> Copy for Message<LogKeyType, LogValueType>
where
    LogKeyType: Copy,
    LogValueType: Copy,
{
}

#[cfg(test)]
mod test {
    use super::{client_message::ClientLogMessage, Message};

    #[test]
    fn test_message_serializability() {
        let messages: Vec<Message<&[u8], &[u8]>> = vec![
            Message::IncomingClientMessage(ClientLogMessage {
                client_id: super::client_message::ClientId(0),
                message_id: super::client_message::MessageId(1),
                operation: super::client_message::ClientMessageOperation::Put {
                    key: b"hello",
                    value: b"world",
                },
            }),
            Message::IncomingClientMessage(ClientLogMessage {
                client_id: super::client_message::ClientId(0),
                message_id: super::client_message::MessageId(1),
                operation: super::client_message::ClientMessageOperation::Put {
                    key: b"foo",
                    value: b"bar",
                },
            }),
        ];

        let bytes = bincode::serialize(&messages).unwrap();
        let _result: Vec<Message<&[u8], &[u8]>> = bincode::deserialize(&bytes).unwrap();
    }
}
