use std::{
    alloc::Allocator,
    hash::Hash,
    ptr::NonNull,
    sync::atomic::{AtomicBool, Ordering},
};

use bincode::Options;
use dpdk::{
    device::eth::dev::{get_port_mtu, EthdevPortId, EventQueueId},
    memory::{
        allocator::{DPDKAllocator, DPDK_ALLOCATOR},
        mbuf::PktMbuf,
    },
    raw::rte_mbuf,
};

use itertools::Itertools;

use serde::{de::DeserializeOwned, Serialize};

use hashbrown::HashMap;
use tracing::instrument;

use crate::{
    message::{
        ack::AckMessage,
        client_message::{ClientLogMessage, ClientMessageOperation},
        Message,
    },
    prefix::{Prefix, PrefixInner},
    state::State,
};

use core::fmt::Debug;

use super::eth::read_from_nic_port_into_buffer;

pub type SpscProducerChannelHandle<T> = rtrb::Producer<T>;
pub type SpscConsumerChannelHandle<T> = rtrb::Consumer<T>;
pub type MpmcProducerChannelHandle<T> = kanal::Sender<T>;
pub type MpmcConsumerChannelHandle<T> = kanal::Receiver<T>;

static SHUTDOWN_FLAG: AtomicBool = AtomicBool::new(false);

/// Parallel
// #[instrument(skip_all)]
pub fn parse_packets<
    LogKeyType,
    LogValueType,
    A: Allocator,
    const NUM_CLIENTS: usize,
    const BUFFER_SIZE: usize,
    const ETHDEV_PORT_ID: EthdevPortId,
    const ETHDEV_QUEUE_ID: EventQueueId,
>(
    mut client_log_message_channels: [SpscProducerChannelHandle<ClientLogMessage<LogKeyType, LogValueType>>;
        NUM_CLIENTS],
) -> Result<(), anyhow::Error>
where
    Vec<Message<LogKeyType, LogValueType>, DPDKAllocator>: Serialize + DeserializeOwned,
    LogKeyType: Eq + Hash + Serialize + DeserializeOwned + core::fmt::Debug + Clone,
    LogValueType: Serialize + DeserializeOwned + core::fmt::Debug + Clone,
{
    let mut buffer = unsafe {
        Box::<[Option<NonNull<rte_mbuf>>; BUFFER_SIZE], DPDKAllocator>::new_zeroed_in(
            DPDK_ALLOCATOR,
        )
        .assume_init()
    };
    for channel in client_log_message_channels.iter() {
        assert!(
            BUFFER_SIZE >= channel.buffer().capacity(),
            "Channel was too small"
        );
    }
    let mut client_log_message_buffer = {
        let mtu = get_port_mtu(ETHDEV_PORT_ID).expect("Failed to get mtu") as usize;
        let max_usable_bytes_in_packet = mtu - 28; // Ether + IPv4 (which is smaller) + UDP
        let max_messages_per_pkt = max_usable_bytes_in_packet
            .div_ceil(std::mem::size_of::<Message<LogKeyType, LogValueType>>());
        Vec::with_capacity_in(max_messages_per_pkt, DPDK_ALLOCATOR)
    };
    loop {
        let num_read = read_from_nic_port_into_buffer::<BUFFER_SIZE, ETHDEV_PORT_ID, ETHDEV_QUEUE_ID>(
            buffer.as_mut(),
        );
        if num_read == 0 && SHUTDOWN_FLAG.load(Ordering::Acquire) {
            tracing::info!("Input channel is abandonded and empty, exiting");
            return Ok(()); // returning causes all of the client log message channels to be abandoned as well.
        } else {
            for pkt in &mut buffer[0..num_read] {
                let mut pkt = pkt.take().unwrap();
                // Safe because buffer[0..num_read] was initalized
                let buf: PktMbuf<Vec<Message<LogKeyType, LogValueType>>> =
                    unsafe { PktMbuf::from(pkt.as_mut()) };
                let data = buf.data_as_byte_slice();
                let msgs: Vec<Message<LogKeyType, LogValueType>> = bincode::options()
                    .reject_trailing_bytes()
                    .with_fixint_encoding()
                    .with_native_endian()
                    .with_limit(10_000) // Larger than the packet size that should be accepted.
                    .deserialize(data)
                    .unwrap();
                drop(buf); // frees the underlying PktMBuf
                msgs.into_iter()
                    .map(|msg| match msg {
                        Message::IncomingClientMessage(msg) => msg,
                        Message::AckMessage(_) => {
                            unimplemented!("Ack message input is not yet supported")
                        }
                    })
                    .group_by(|msg| msg.client_id)
                    .into_iter()
                    .for_each(|(client_id, messages)| {
                        let mut max_allowed_index: usize = 0;
                        let mut iter = messages.enumerate();
                        while max_allowed_index < client_log_message_buffer.len() {
                            match iter.next() {
                                Some((index, msg)) => {
                                    max_allowed_index = index;
                                    client_log_message_buffer[index] = msg;
                                }
                                None => unreachable!(),
                            }
                        }
                        for (_, msg) in iter {
                            client_log_message_buffer.push(msg);
                        }
                        let channel = client_log_message_channels
                            .get_mut(client_id.0 as usize)
                            .unwrap();
                        while channel.slots() < max_allowed_index + 1 {} //spin
                        let mut chunk = channel.write_chunk_uninit(max_allowed_index + 1).unwrap();
                        let (first, second) = chunk.as_mut_slices();
                        for index in 0..(first.len().max(max_allowed_index + 1)) {
                            first[index].write(client_log_message_buffer[index].clone());
                        }
                        if first.len() < max_allowed_index + 1 {
                            for index in 0..(second.len().max(max_allowed_index + 1 - first.len()))
                            {
                                first[index]
                                    .write(client_log_message_buffer[index + first.len()].clone());
                            }
                        }
                    });
            }
        }
    }
}

#[instrument(skip_all, fields(clients = ?(CLIENT_ID_OFFSET..(CLIENT_ID_OFFSET + NUM_CLIENTS))))]
pub fn apply_messages_pipeline<
    LogKeyType,
    LogValueType,
    const CLIENT_ID_OFFSET: usize,
    const NUM_CLIENTS: usize,
    A,
    S,
>(
    client_channels: &mut [SpscConsumerChannelHandle<ClientLogMessage<LogKeyType, LogValueType>>],
    mut prefix_channel: SpscConsumerChannelHandle<Prefix<NUM_CLIENTS>>,
    mut output_channel: SpscProducerChannelHandle<AckMessage<LogValueType>>,
    allocator: A,
) -> Result<impl State<LogKeyType, LogValueType, A>, anyhow::Error>
where
    LogKeyType: Eq + Hash + Clone + Debug,
    LogValueType: Clone + Debug,
    PrefixInner<NUM_CLIENTS>: Serialize + DeserializeOwned,
    A: Allocator + Clone,
    S: std::hash::BuildHasher + Default,
{
    let mut state = HashMap::<LogKeyType, LogValueType, _, _>::with_capacity_and_hasher_in(
        1000,
        S::default(),
        allocator,
    );
    let mut current_prefix = Prefix::<NUM_CLIENTS>::new(0);

    loop {
        match prefix_channel.pop() {
            Ok(new_prefix) => {
                let _prefix_span = tracing::trace_span!("prefix", prefix = ?new_prefix).entered();
                if current_prefix.id + 1 != new_prefix.id {
                    debug_assert_eq!(0, current_prefix.id);
                }
                let delta_prefix = current_prefix.delta_to(&new_prefix);
                tracing::trace!(event = "Delta prefix", delta_prefix = ?delta_prefix);
                for client_id in 0..NUM_CLIENTS {
                    apply_messages_for_client_id::<
                        LogKeyType,
                        LogValueType,
                        CLIENT_ID_OFFSET,
                        NUM_CLIENTS,
                        A,
                        S,
                    >(
                        client_id,
                        delta_prefix,
                        client_channels,
                        &mut output_channel,
                        &mut state,
                    );
                }
                current_prefix = new_prefix;
            }
            Err(_) => {
                if prefix_channel.is_abandoned() {
                    return Ok(state);
                }
            }
        }
    }
}

#[instrument(
    level = "debug",
    skip(delta_prefix, client_channels, output_channel, state)
)]
fn apply_messages_for_client_id<
    LogKeyType,
    LogValueType,
    const CLIENT_ID_OFFSET: usize,
    const NUM_CLIENTS: usize,
    A,
    S: std::hash::BuildHasher,
>(
    client_id: usize,
    delta_prefix: crate::prefix::PrefixDelta<NUM_CLIENTS>,
    client_channels: &mut [rtrb::Consumer<ClientLogMessage<LogKeyType, LogValueType>>],
    output_channel: &mut rtrb::Producer<AckMessage<LogValueType>>,
    state: &mut HashMap<LogKeyType, LogValueType, S, A>,
) where
    LogKeyType: Eq + Hash + Clone + Debug,
    LogValueType: Clone + Debug,
    PrefixInner<NUM_CLIENTS>: Serialize + DeserializeOwned,
    A: Allocator + Clone,
{
    let client_id_offset = client_id - CLIENT_ID_OFFSET;
    let mut messages_to_apply = delta_prefix.states[client_id_offset].0 as usize;
    tracing::trace!(messages_to_apply);
    while messages_to_apply > 0 {
        let queue = &mut client_channels[client_id];
        let messages_to_apply_this_round = messages_to_apply.min(queue.slots());
        let read_chunk = queue.read_chunk(messages_to_apply_this_round).unwrap();
        let write_chunk = output_channel
            .write_chunk_uninit(messages_to_apply_this_round)
            .unwrap();
        #[cfg(debug_assertions)]
        {
            // make sure everything is in order
            let (first, second) = read_chunk.as_slices();
            for (a, b) in first.iter().chain(second).tuple_windows() {
                assert_eq!(a.client_id, b.client_id);
                assert_eq!(a.message_id.0 + 1, b.message_id.0);
            }
        }
        let applied_messages = read_chunk
            .into_iter()
            .map(|msg| apply_message_to_state(msg, state));
        let messages_written = write_chunk.fill_from_iter(applied_messages);
        if messages_written > 0 {
            tracing::trace!(event = "Applied messages", count = messages_written);
            messages_to_apply -= messages_written;
        }
    }
}

#[inline]
#[instrument(level = "debug", skip(state), fields(msg = %msg))]
pub fn apply_message_to_state<
    LogKeyType,
    LogValueType,
    A: Allocator,
    S: State<LogKeyType, LogValueType, A>,
>(
    msg: ClientLogMessage<LogKeyType, LogValueType>,
    state: &mut S,
) -> AckMessage<LogValueType>
where
    LogKeyType: Clone + Debug,
    LogValueType: Clone + Debug,
{
    tracing::trace!(?msg);
    let ack = match msg.operation {
        ClientMessageOperation::Get { key } => {
            let result = state.get(&key);
            AckMessage {
                client_id: msg.client_id,
                message_id: msg.message_id,
                extension: crate::message::ack::AckMessageExtensions::Get(result.cloned()),
            }
        }
        ClientMessageOperation::Put { key, value } => {
            let prev = state.put(key, value);
            AckMessage {
                client_id: msg.client_id,
                message_id: msg.message_id,
                extension: crate::message::ack::AckMessageExtensions::Put(prev),
            }
        }
        ClientMessageOperation::Delete { key } => {
            let removed = state.delete(&key);
            AckMessage {
                client_id: msg.client_id,
                message_id: msg.message_id,
                extension: crate::message::ack::AckMessageExtensions::Delete(removed),
            }
        }
    };
    tracing::trace!(?ack);
    ack
}

#[cfg(test)]
mod test {
    use std::{
        alloc::Global,
        fmt::Debug,
        sync::{
            atomic::{fence, AtomicU32, Ordering},
            Arc,
        },
    };

    use bitvec::store::BitStore;
    use hashbrown::hash_map::DefaultHashBuilder;

    use crate::{
        message::{
            ack::AckMessage,
            client_message::{ClientId, ClientLogMessage, MessageId},
        },
        prefix::Prefix,
        state::State,
        sync_provider::{test::TestLocalSyncProvider, SyncProvider},
        workers::{
            order_messages::order_messages,
            pipeline::{
                apply_messages_pipeline, test::ordering::get_sequence_of_n_shuffled_messages,
            },
        },
    };

    use super::apply_message_to_state;

    mod helpers {
        use crate::message::{
            ack::{AckMessage, AckMessageExtensions},
            client_message::{ClientId, ClientLogMessage, ClientMessageOperation, MessageId},
        };

        pub fn make_put_pair<LogKeyType, LogValueType>(
            key: LogKeyType,
            value: LogValueType,
            r: Option<LogValueType>,
        ) -> (
            ClientLogMessage<LogKeyType, LogValueType>,
            AckMessage<LogValueType>,
        ) {
            (
                ClientLogMessage {
                    client_id: ClientId(0),
                    message_id: MessageId(0),
                    operation: ClientMessageOperation::Put { key, value },
                },
                AckMessage {
                    client_id: ClientId(0),
                    message_id: MessageId(0),
                    extension: AckMessageExtensions::Put(r),
                },
            )
        }

        pub fn make_put_pair_with_msg_id<LogKeyType, LogValueType>(
            message_id: u32,
            key: LogKeyType,
            value: LogValueType,
            r: Option<LogValueType>,
        ) -> (
            ClientLogMessage<LogKeyType, LogValueType>,
            AckMessage<LogValueType>,
        ) {
            let (mut c, mut a) = make_put_pair(key, value, r);
            c.message_id = MessageId(message_id);
            a.message_id = MessageId(message_id);
            (c, a)
        }

        pub fn make_get_pair<LogKeyType, LogValueType>(
            key: LogKeyType,
            r: Option<LogValueType>,
        ) -> (
            ClientLogMessage<LogKeyType, LogValueType>,
            AckMessage<LogValueType>,
        ) {
            (
                ClientLogMessage {
                    client_id: ClientId(0),
                    message_id: MessageId(0),
                    operation: ClientMessageOperation::Get { key },
                },
                AckMessage {
                    client_id: ClientId(0),
                    message_id: MessageId(0),
                    extension: AckMessageExtensions::Get(r),
                },
            )
        }

        pub fn make_get_pair_with_msg_id<LogKeyType, LogValueType>(
            message_id: u32,
            key: LogKeyType,
            r: Option<LogValueType>,
        ) -> (
            ClientLogMessage<LogKeyType, LogValueType>,
            AckMessage<LogValueType>,
        ) {
            let (mut c, mut a) = make_get_pair(key, r);
            c.message_id = MessageId(message_id);
            a.message_id = MessageId(message_id);
            (c, a)
        }

        pub fn make_delete_pair<LogKeyType, LogValueType>(
            key: LogKeyType,
            r: Option<LogValueType>,
        ) -> (
            ClientLogMessage<LogKeyType, LogValueType>,
            AckMessage<LogValueType>,
        ) {
            (
                ClientLogMessage {
                    client_id: ClientId(0),
                    message_id: MessageId(0),
                    operation: ClientMessageOperation::Delete { key },
                },
                AckMessage {
                    client_id: ClientId(0),
                    message_id: MessageId(0),
                    extension: AckMessageExtensions::Delete(r),
                },
            )
        }

        pub fn make_delete_pair_with_msg_id<LogKeyType, LogValueType>(
            message_id: u32,
            key: LogKeyType,
            r: Option<LogValueType>,
        ) -> (
            ClientLogMessage<LogKeyType, LogValueType>,
            AckMessage<LogValueType>,
        ) {
            let (mut c, mut a) = make_delete_pair(key, r);
            c.message_id = MessageId(message_id);
            a.message_id = MessageId(message_id);
            (c, a)
        }
    }

    pub fn apply_messages_to_state<
        LogKeyType,
        LogValueType,
        StateImpl: State<LogKeyType, LogValueType> + Default,
    >(
        mut state: StateImpl,
        generator: impl Iterator<
            Item = (
                ClientLogMessage<LogKeyType, LogValueType>,
                AckMessage<LogValueType>,
            ),
        >,
    ) -> StateImpl
    where
        LogKeyType: Clone + Debug + PartialEq,
        LogValueType: Clone + Debug + PartialEq,
    {
        for (msg, expected) in generator {
            let resulting_ack = apply_message_to_state(msg, &mut state);
            assert_eq!(
                expected, resulting_ack,
                "Expected and resulting acks were different"
            );
        }
        state
    }

    pub mod ordering {
        use std::sync::{
            atomic::{AtomicU32, Ordering},
            Arc,
        };

        use arbitrary::{Arbitrary, Unstructured};
        use bitvec::store::BitStore;
        use itertools::Itertools;
        use rand::{seq::SliceRandom, Rng};

        use crate::{
            message::client_message::{ClientId, ClientLogMessage, MessageId},
            workers::order_messages::order_messages,
        };

        pub fn get_sequence_of_n_messages<
            LogKeyType: for<'a> Arbitrary<'a> + Clone,
            LogValueType: for<'a> Arbitrary<'a> + Clone,
            const NUM_MESSAGES: usize,
        >(
            mut msg_id: u32,
        ) -> Vec<ClientLogMessage<LogKeyType, LogValueType>> {
            let num_bytes =
                std::mem::size_of::<ClientLogMessage<LogKeyType, LogValueType>>() * NUM_MESSAGES;
            let mut rand = rand::thread_rng();
            let data = (0..num_bytes)
                .map(|_| rand.gen::<u64>())
                .flat_map(u64::to_ne_bytes)
                .collect_vec();
            let mut unstructured = Unstructured::new(&data);
            let mut v = Vec::with_capacity(NUM_MESSAGES);
            (0..NUM_MESSAGES).for_each(|_| {
                let mut msg: ClientLogMessage<LogKeyType, LogValueType> =
                    unstructured.arbitrary().unwrap();
                msg.client_id = ClientId(0);
                msg.message_id = MessageId(msg_id);
                v.push(msg);
                msg_id += 1;
            });
            v
        }

        pub fn get_sequence_of_n_shuffled_messages<
            LogKeyType: for<'a> Arbitrary<'a> + Clone,
            LogValueType: for<'a> Arbitrary<'a> + Clone,
            const NUM_MESSAGES: usize,
        >(
            starting_message_id: u32,
        ) -> Vec<ClientLogMessage<LogKeyType, LogValueType>> {
            let mut random = rand::thread_rng();
            let mut messages = get_sequence_of_n_messages::<LogKeyType, LogValueType, NUM_MESSAGES>(
                starting_message_id,
            );
            messages.shuffle(&mut random);
            messages
        }

        #[test]
        fn single_client_msg_reorder_test() {
            const NUM_MESSAGES: usize = 1_000;
            let (mut pipeline_write_handle, input_channel) = rtrb::RingBuffer::new(NUM_MESSAGES);
            let (output_channel, mut pipeline_read_handle) = rtrb::RingBuffer::new(NUM_MESSAGES);
            let next_holder = Arc::new(AtomicU32::new(0));
            let next_holder_ref = next_holder.clone();
            let join_handle = std::thread::spawn(move || {
                order_messages(input_channel, output_channel, next_holder_ref, ClientId(0))
            });
            let messages = get_sequence_of_n_shuffled_messages::<u32, u32, NUM_MESSAGES>(0);
            let chunk = pipeline_write_handle
                .write_chunk_uninit(NUM_MESSAGES)
                .unwrap();
            let num_written = chunk.fill_from_iter(messages);
            assert_eq!(NUM_MESSAGES, num_written, "Not all messages were written.");
            drop(pipeline_write_handle);
            join_handle.join().unwrap().unwrap();
            let read_chunk = pipeline_read_handle.read_chunk(NUM_MESSAGES).unwrap();
            assert_eq!(
                NUM_MESSAGES,
                read_chunk.into_iter().count(),
                "Not all messages were ordered and sent onward."
            );
            assert!(
                pipeline_read_handle.is_empty(),
                "Did not consumer entire buffer"
            );
            assert_eq!(
                NUM_MESSAGES as u32,
                next_holder.load(Ordering::SeqCst),
                "Next holder was not updated"
            )
        }

        fn single_client_msg_reorder_test_multiple_round<
            LogKeyType,
            LogValueType,
            const NUM_MESSAGES: usize,
            const NUM_ROUNDS: usize,
        >(
            messages_function: fn(u32) -> Vec<ClientLogMessage<LogKeyType, LogValueType>>,
        ) where
            for<'a> LogKeyType: 'a + Clone + core::fmt::Debug + Send,
            for<'a> LogValueType: 'a + Clone + core::fmt::Debug + Send,
        {
            let (mut pipeline_write_handle, input_channel) = rtrb::RingBuffer::new(NUM_MESSAGES);
            let (output_channel, mut pipeline_read_handle) = rtrb::RingBuffer::new(NUM_MESSAGES);
            let next_holder = Arc::new(AtomicU32::new(0));
            let next_holder_ref = next_holder.clone();
            let join_handle = std::thread::spawn(move || {
                order_messages(input_channel, output_channel, next_holder_ref, ClientId(0))
            });

            for round in 0..NUM_ROUNDS {
                let messages = messages_function((NUM_MESSAGES * round) as u32);
                let chunk = pipeline_write_handle
                    .write_chunk_uninit(NUM_MESSAGES)
                    .unwrap();
                let num_written = chunk.fill_from_iter(messages.clone());
                assert_eq!(
                    NUM_MESSAGES, num_written,
                    "Not all messages were written in round {round}."
                );
                let mut remaining_messages = NUM_MESSAGES;
                while remaining_messages > 0 {
                    while pipeline_read_handle.is_empty() {}
                    let read_chunk = pipeline_read_handle
                        .read_chunk(pipeline_read_handle.slots())
                        .unwrap();
                    remaining_messages -= read_chunk.into_iter().count();
                }
                assert!(
                    pipeline_read_handle.is_empty(),
                    "Queue was not emptied in round {round}"
                );
                let order_messages_processed = next_holder.load_value();
                let expected_messages_processed = ((round + 1) * NUM_MESSAGES) as u32;
                assert_eq!(
                    expected_messages_processed,
                    order_messages_processed,
                    "Next holder was {order_messages_processed}, expected {expected_messages_processed} in round {round}"
                )
            }

            drop(pipeline_write_handle);
            join_handle.join().unwrap().unwrap();
            assert!(
                pipeline_read_handle.is_empty(),
                "Did not consumer entire buffer"
            );
        }

        #[test_log::test]
        fn single_client_msg_reorder_test_multiple_round_randomized() {
            const NUM_MESSAGES: usize = 10_000;
            const NUM_ROUNDS: usize = 10;
            single_client_msg_reorder_test_multiple_round::<u32, u32, NUM_MESSAGES, NUM_ROUNDS>(
                get_sequence_of_n_shuffled_messages::<u32, u32, NUM_MESSAGES>,
            )
        }

        #[test_log::test]
        fn single_client_msg_reorder_test_multiple_round_sorted_input() {
            const NUM_MESSAGES: usize = 10_000;
            const NUM_ROUNDS: usize = 10;
            single_client_msg_reorder_test_multiple_round::<u32, u32, NUM_MESSAGES, NUM_ROUNDS>(
                get_sequence_of_n_messages::<u32, u32, NUM_MESSAGES>,
            )
        }
    }

    pub mod applying {
        use std::alloc::Global;

        use super::{apply_messages_to_state, helpers};

        #[test]
        fn test_apply_simple_messages_results_in_correct_acks() {
            let test_data = [
                helpers::make_put_pair_with_msg_id(0, 1, 2, None),
                helpers::make_put_pair_with_msg_id(1, 1, 3, Some(2)),
                helpers::make_put_pair_with_msg_id(2, 1, 4, Some(3)),
                helpers::make_put_pair_with_msg_id(3, 1, 5, Some(4)),
                helpers::make_delete_pair_with_msg_id(4, 1, Some(5)),
                helpers::make_get_pair_with_msg_id(5, 1, None),
            ];
            let state = hashbrown::HashMap::<u32, u32>::new_in(Global::default());
            let _state = apply_messages_to_state::<u32, u32, hashbrown::HashMap<u32, u32>>(
                state,
                test_data.into_iter(),
            );
        }
    }

    #[test_log::test]
    fn order_apply_messages() {
        const NUM_MESSAGES: usize = 1_000;
        let (mut pipeline_write_handle, order_input) = rtrb::RingBuffer::new(NUM_MESSAGES);
        let (order_output, apply_input) = rtrb::RingBuffer::new(NUM_MESSAGES);
        let (apply_output, mut pipeline_read_handle) = rtrb::RingBuffer::new(NUM_MESSAGES);

        let (mut prefix_write_handle, apply_prefix_input) = rtrb::RingBuffer::new(NUM_MESSAGES);

        let next_holder = Arc::new(AtomicU32::new(0));
        let next_holder_ref = next_holder;
        let order_join_handle = std::thread::spawn(move || {
            order_messages(order_input, order_output, next_holder_ref, ClientId(0))
        });
        let apply_handle = std::thread::spawn(move || {
            let mut client_channels = [apply_input];
            apply_messages_pipeline::<u32, u32, 0, 1, Global, DefaultHashBuilder>(
                &mut client_channels,
                apply_prefix_input,
                apply_output,
                Global::default(),
            )
        });
        let messages = get_sequence_of_n_shuffled_messages::<u32, u32, NUM_MESSAGES>(0);
        let mut current_prefix = Prefix::<1>::new(0);
        current_prefix.states[0] += MessageId(NUM_MESSAGES as u32);

        prefix_write_handle.push(current_prefix).unwrap();
        let chunk = pipeline_write_handle
            .write_chunk_uninit(NUM_MESSAGES)
            .unwrap();
        let num_written = chunk.fill_from_iter(messages);
        assert_eq!(NUM_MESSAGES, num_written, "Not all messages were written.");
        let mut to_get = NUM_MESSAGES;
        while to_get > 0 {
            if pipeline_read_handle.pop().is_ok() {
                to_get -= 1;
            }
        }
        current_prefix.states[0] += MessageId(NUM_MESSAGES as u32);
        drop(pipeline_write_handle);
        drop(prefix_write_handle);
        order_join_handle.join().unwrap().unwrap();
        let _state = apply_handle.join().unwrap().unwrap();
    }

    #[test_log::test]
    fn order_apply_messages_with_prefixes() {
        const NUM_MESSAGES: usize = 10;
        const NUM_ROUNDS: usize = 4;
        let (mut pipeline_write_handle, order_input) = rtrb::RingBuffer::new(NUM_MESSAGES);
        let (order_output, apply_input) = rtrb::RingBuffer::new(NUM_MESSAGES);
        let (apply_output, mut pipeline_read_handle) = rtrb::RingBuffer::new(NUM_MESSAGES);

        let (mut prefix_write_handle, apply_prefix_input) = rtrb::RingBuffer::new(NUM_MESSAGES);

        let next_holder_for_order_handle = Arc::new(AtomicU32::new(0));
        let next_holder_for_order_handle_ref = next_holder_for_order_handle.clone();

        let order_join_handle = std::thread::spawn(move || {
            order_messages(
                order_input,
                order_output,
                next_holder_for_order_handle,
                ClientId(0),
            )
        });

        let apply_handle = std::thread::spawn(move || {
            let mut client_channels = [apply_input];
            apply_messages_pipeline::<u32, u32, 0, 1, Global, DefaultHashBuilder>(
                &mut client_channels,
                apply_prefix_input,
                apply_output,
                Global::default(),
            )
        });

        let (prefix_input_handle, ticker_input) = crossbeam_channel::bounded(NUM_ROUNDS);

        let ticker = std::thread::spawn(move || {
            let mut sync_provider = TestLocalSyncProvider {
                channel: ticker_input,
            };

            for _ in 0..NUM_ROUNDS {
                prefix_write_handle
                    .push(sync_provider.tick().unwrap())
                    .unwrap();
            }
        });

        for round in 0..NUM_ROUNDS {
            fence(Ordering::SeqCst);
            assert_eq!(
                NUM_MESSAGES,
                pipeline_write_handle.slots(),
                "pipeline write handle not empty on round {round}",
            );
            if !pipeline_read_handle.is_empty() {
                assert_eq!(
                    0,
                    pipeline_read_handle.slots(),
                    "Pipeline read handle not empty on round {round}"
                );
            }

            let messages = get_sequence_of_n_shuffled_messages::<u32, u32, NUM_MESSAGES>(
                (round * NUM_MESSAGES) as u32,
            );
            let chunk = pipeline_write_handle
                .write_chunk_uninit(NUM_MESSAGES)
                .unwrap();
            let num_written = chunk.fill_from_iter(messages.clone());
            assert_eq!(NUM_MESSAGES, num_written, "Not all messages were written.");
            assert_eq!(
                0,
                pipeline_read_handle.slots(),
                "Not everything was ready from the pipeline"
            );
            prefix_input_handle
                .send(Prefix {
                    id: round,
                    states: [MessageId((NUM_MESSAGES * (round + 1)) as u32)],
                })
                .unwrap();
            let mut to_get = NUM_MESSAGES;
            while to_get > 0 {
                while pipeline_read_handle.slots() == 0 {}
                let read_chunk = pipeline_read_handle
                    .read_chunk(pipeline_read_handle.slots())
                    .unwrap();
                let (first, second) = read_chunk.as_slices();
                let num_read = first.len() + second.len();
                read_chunk.commit_all();
                to_get -= num_read;
                tracing::debug!(
                    event = "got acks",
                    remaining_this_round = to_get,
                    num_read = num_read
                )
            }
            let num_ordered = next_holder_for_order_handle_ref.load_value();
            let expected_num_ordered = ((round + 1) * NUM_MESSAGES) as u32;

            assert_eq!(
                expected_num_ordered,
                num_ordered,
                "Not all messages were processed in round {round}, only {num_ordered} out of {expected_num_ordered} were.",
            );
        }
        drop(pipeline_write_handle);
        order_join_handle.join().unwrap().unwrap();
        let _state = apply_handle.join().unwrap().unwrap();
        ticker.join().unwrap();
    }
}
