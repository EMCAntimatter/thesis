mod local_sync_provider;

use serde::{de::DeserializeOwned, Serialize};

use crate::prefix::{Prefix, PrefixInner};

pub trait SyncProvider<const NUM_CLIENTS: usize>
where
    PrefixInner<NUM_CLIENTS>: Serialize + DeserializeOwned,
{
    type Error;

    fn tick(&mut self) -> Result<Prefix<NUM_CLIENTS>, Self::Error>;
}

pub use local_sync_provider::*;
