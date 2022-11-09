use std::{
    fmt::Display,
    ops::{Add, AddAssign, Sub},
};

use arbitrary::Arbitrary;
use num::Integer;
use serde::{Deserialize, Serialize};

pub type MessageCount = u16;
pub type MessageTimestamp = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Arbitrary)]
pub struct ClientId(pub u32);

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Arbitrary,
)]
pub struct MessageId(pub u32);

impl MessageId {
    pub fn next(&self) -> MessageId {
        let (next_value, _) = self.0.overflowing_add(1);
        MessageId(next_value)
    }
}

impl Add for MessageId {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl<T: num::Integer + num::Unsigned + Into<u32>> Add<T> for MessageId {
    type Output = Self;

    fn add(self, rhs: T) -> Self::Output {
        Self(self.0 + rhs.into())
    }
}

impl AddAssign for MessageId {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0
    }
}

impl AddAssign<u16> for MessageId {
    fn add_assign(&mut self, rhs: u16) {
        self.0 += rhs as u32
    }
}

impl Sub for MessageId {
    type Output = MessageId;

    fn sub(self, rhs: Self) -> Self::Output {
        MessageId(self.0 - rhs.0)
    }
}

impl PartialEq<u16> for MessageId {
    fn eq(&self, other: &u16) -> bool {
        self.0 == *other as u32
    }
}

#[derive(Clone, Serialize, Deserialize, Debug, Arbitrary)]
pub enum ClientMessageOperation<LogKeyType, LogValueType> {
    Get {
        key: LogKeyType,
    },
    Put {
        key: LogKeyType,
        value: LogValueType,
    },
    Delete {
        key: LogKeyType,
    },
}

impl<LogKeyType, LogValueType> ClientMessageOperation<LogKeyType, LogValueType> {
    #[inline]
    pub fn get_key(&self) -> &LogKeyType {
        match self {
            ClientMessageOperation::Get { key } => key,
            ClientMessageOperation::Put { key, value: _ } => key,
            ClientMessageOperation::Delete { key } => key,
        }
    }
}

impl<LogKeyType, LogValueType> Copy for ClientMessageOperation<LogKeyType, LogValueType>
where
    LogKeyType: Copy,
    LogValueType: Copy,
{
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Arbitrary)]
pub struct ClientLogMessage<LogKeyType, LogValueType> {
    pub client_id: ClientId,
    pub message_id: MessageId,
    pub operation: ClientMessageOperation<LogKeyType, LogValueType>,
}

impl<LogKeyType, LogValueType> Display for ClientLogMessage<LogKeyType, LogValueType> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "(client_id: {}, message_id: {})",
            self.client_id.0, self.message_id.0
        )
    }
}
