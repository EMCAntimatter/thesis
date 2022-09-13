use serde::{Deserialize, Serialize};

use super::{ClientId, ClientMessage, MessageCount, MessageId, MessageTimestamp};

#[repr(C)]
#[derive(Serialize, Deserialize, Clone)]
pub struct ClientMessagePacketHeader {
    pub timestamp: MessageTimestamp,
    pub client_id: ClientId,
    pub number_of_messages: MessageCount,
    pub starting_message_id: MessageId,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ClientMessagePacket {
    pub header: ClientMessagePacketHeader,
    pub messages: Vec<ClientMessage>,
}

#[cfg(test)]
mod test {
    use crate::message::{ClientId, ClientMessage, MessageId};

    use super::{ClientMessagePacket, ClientMessagePacketHeader};

    #[test]
    fn test_serialization_format() {
        let client_packet = ClientMessagePacket {
            header: ClientMessagePacketHeader {
                timestamp: 0,
                client_id: ClientId(0),
                number_of_messages: 1,
                starting_message_id: MessageId(0),
            },
            messages: vec![
                ClientMessage::Set { key: 0, value: 0 },
                ClientMessage::Get { key: 0 },
            ],
        };

        let result = bincode::serialize(&client_packet).unwrap();
        println!("{result:?}")
    }
}
