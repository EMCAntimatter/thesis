use std::{
    alloc::{Allocator, Global},
    hash::{Hash, Hasher},
    sync::Arc,
};

use dpdk::raw::rte_mbuf;
use partitioned_swiss_table::table::{PartitionedHashMap, PartitionedHashMapHandle};
use rtrb::{Consumer, Producer};
use serde::{de::DeserializeOwned, Serialize};

use crate::message::{ack::AckMessage, client_message::ClientLogMessage};


pub type RtrbChanPair<T> = (Producer<T>, Consumer<T>);
pub type KanalChanPair<T> = (kanal::Sender<T>, kanal::Receiver<T>);

pub struct DatabaseComponents<
    LogKeyType,
    LogValueType,
    const NUM_PARTITIONS: usize = 1,
    const NUM_CLIENTS: usize = 1,
    Alloc = Global,
    HasherType = ahash::AHasher,
> where
    LogKeyType: Eq + Hash + Serialize + DeserializeOwned + core::fmt::Debug,
    LogValueType: Serialize + DeserializeOwned + core::fmt::Debug,
    Alloc: Allocator + Clone,
    HasherType: Hasher + Default,
{
    pub table: Arc<PartitionedHashMap<LogKeyType, LogValueType, NUM_PARTITIONS, Alloc, HasherType>>,
    pub handles:
        [Option<PartitionedHashMapHandle<LogKeyType, LogValueType, NUM_PARTITIONS, Alloc, HasherType>>; NUM_PARTITIONS],
    pub order_input_channels: [RtrbChanPair<&'static mut rte_mbuf> ; NUM_CLIENTS],
    pub apply_input_channels: [KanalChanPair<ClientLogMessage<LogKeyType, LogValueType>>; NUM_PARTITIONS],
    pub apply_output_channels: [RtrbChanPair<AckMessage<LogValueType>>; NUM_PARTITIONS],
}

pub trait Database<
    LogKeyType,
    LogValueType,
    const IO_INPUT_CHANNEL_CAPACITY: usize = 400_000,
    const NUM_PARTITIONS: usize = 1,
    const NUM_CLIENTS: usize = 1,
    Alloc = Global,
    HasherType = ahash::AHasher,
> where
    LogKeyType: Eq + Hash + Serialize + DeserializeOwned + core::fmt::Debug,
    LogValueType: Serialize + DeserializeOwned + core::fmt::Debug,
    Alloc: Allocator + Clone + Default,
    HasherType: Hasher + Default,
{
    fn new_components(
    ) -> DatabaseComponents<LogKeyType, LogValueType, NUM_PARTITIONS, NUM_CLIENTS, Alloc, HasherType>
    {
        let table = PartitionedHashMap::with_capacity_and_hasher_in(100_000, HasherType::default(), Alloc::default());
        let handles = PartitionedHashMap::create_all_handles(&table);
        let order_input_channels =
            std::array::from_fn(|_| rtrb::RingBuffer::new(IO_INPUT_CHANNEL_CAPACITY));
        let apply_input_channels =
            std::array::from_fn(|_| kanal::bounded(IO_INPUT_CHANNEL_CAPACITY));
        let apply_output_channels =
            std::array::from_fn(|_| rtrb::RingBuffer::new(IO_INPUT_CHANNEL_CAPACITY));
        DatabaseComponents {
            table,
            handles,
            order_input_channels,
            apply_input_channels,
            apply_output_channels
        }
    }
}
