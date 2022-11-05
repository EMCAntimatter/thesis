use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::message::client_message::MessageId;

pub type PrefixInner<const NUM_CLIENTS: usize> = [MessageId; NUM_CLIENTS];

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub struct Prefix<const NUM_CLIENTS: usize>
where
    PrefixInner<NUM_CLIENTS>: Serialize + DeserializeOwned,
{
    pub id: usize,
    pub states: PrefixInner<NUM_CLIENTS>,
}

impl<const NUM_CLIENTS: usize> Prefix<NUM_CLIENTS>
where
    PrefixInner<NUM_CLIENTS>: Serialize + DeserializeOwned,
{
    pub fn new(id: usize) -> Self {
        Self {
            id,
            states: [MessageId(0); NUM_CLIENTS],
        }
    }

    /// Both prefixes must have the same ID, otherwise None will be returned
    pub fn merge(&self, other: &Self) -> Option<Self> {
        if self.id != other.id {
            return None;
        }
        let merged_states = self.states.zip(other.states).map(|(a, b)| a.min(b));
        Some(Self {
            id: self.id,
            states: merged_states,
        })
    }

    pub fn delta_to(&self, other: &Self) -> PrefixDelta<NUM_CLIENTS> {
        let delta_states = self.states.zip(other.states).map(|(a, b)| b - a);
        PrefixDelta {
            states: delta_states,
        }
    }
}

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub struct PrefixDelta<const NUM_CLIENTS: usize>
where
    PrefixInner<NUM_CLIENTS>: Serialize + DeserializeOwned,
{
    pub states: PrefixInner<NUM_CLIENTS>,
}
