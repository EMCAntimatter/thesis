use std::{
    convert::Infallible,
    sync::atomic::{AtomicU32, Ordering},
};

use serde::{de::DeserializeOwned, Serialize};

use crate::{
    message::client_message::MessageId,
    prefix::{Prefix, PrefixInner},
};

use super::SyncProvider;

///
/// Fakes synchronization to allow for non-replicated tests
///
pub struct LocalSyncProvider<const NUM_CLIENTS: usize> {
    id: usize,
    message_states: [&'static AtomicU32; NUM_CLIENTS],
}

impl<const NUM_CLIENTS: usize> SyncProvider<NUM_CLIENTS> for LocalSyncProvider<NUM_CLIENTS>
where
    PrefixInner<NUM_CLIENTS>: Serialize + DeserializeOwned,
{
    type Error = Infallible;

    fn tick(&mut self) -> Result<Prefix<NUM_CLIENTS>, Self::Error> {
        self.id += 1;
        let prefix_array = self
            .message_states
            .map(|prefix| MessageId(prefix.load(Ordering::Acquire)));
        Ok(Prefix {
            id: self.id,
            states: prefix_array,
        })
    }
}

#[cfg(test)]
pub mod test {
    

    use crossbeam_channel::RecvError;
    use serde::{de::DeserializeOwned, Serialize};

    use crate::{
        prefix::{Prefix, PrefixInner},
        sync_provider::SyncProvider,
    };

    ///
    /// Fakes synchronization to allow for non-replicated tests
    ///
    pub struct TestLocalSyncProvider<const NUM_CLIENTS: usize>
    where
        PrefixInner<NUM_CLIENTS>: Serialize + DeserializeOwned,
    {
        pub channel: crossbeam_channel::Receiver<Prefix<NUM_CLIENTS>>,
    }

    impl<const NUM_CLIENTS: usize> SyncProvider<NUM_CLIENTS> for TestLocalSyncProvider<NUM_CLIENTS>
    where
        PrefixInner<NUM_CLIENTS>: Serialize + DeserializeOwned,
    {
        type Error = RecvError;

        fn tick(&mut self) -> Result<Prefix<NUM_CLIENTS>, Self::Error> {
            self.channel.recv()
        }
    }
}
