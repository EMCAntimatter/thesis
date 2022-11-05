use std::{
    alloc::Allocator,
    collections::LinkedList,
    hash::Hash,
    sync::atomic::{AtomicU32, Ordering},
};

use bincode::Options;
use dpdk::{
    memory::{allocator::DPDKAllocator, mbuf::PktMbuf},
    raw::rte_mbuf,
};

use itertools::Itertools;

use serde::{de::DeserializeOwned, Serialize};

use hashbrown::HashMap;
use tracing::instrument;

use crate::{
    message::{
        ack::AckMessage,
        client_message::{ClientId, ClientLogMessage, ClientMessageOperation},
        Message,
    },
    prefix::{Prefix, PrefixInner},
    state::State,
};

use core::fmt::Debug;

use super::eth::read_packets_from_nic_port_0_into_channel;

pub type SpscProducerChannelHandle<T> = rtrb::Producer<T>;
pub type SpscConsumerChannelHandle<T> = rtrb::Consumer<T>;
pub type MpmcProducerChannelHandle<T> = kanal::Sender<T>;
pub type MpmcConsumerChannelHandle<T> = kanal::Receiver<T>;

/// Parallel if NIC supports it
pub fn accept_packets<const SIZE: usize>(
    output_channel: SpscProducerChannelHandle<&'static mut rte_mbuf>,
) {
    read_packets_from_nic_port_0_into_channel::<SIZE>(output_channel)
}

/// Parallel
#[instrument(skip_all)]
pub fn parse_packets<LogKeyType, LogValueType, A: Allocator, const NUM_CLIENTS: usize>(
    mut input_channel: SpscConsumerChannelHandle<&'static mut rte_mbuf>,
    mut client_log_message_channels: [SpscProducerChannelHandle<ClientLogMessage<LogKeyType, LogValueType>>;
        NUM_CLIENTS],
) -> Result<(), anyhow::Error>
where
    Vec<Message<LogKeyType, LogValueType>, DPDKAllocator>: Serialize + DeserializeOwned,
    LogKeyType: Eq + Hash + Serialize + DeserializeOwned + core::fmt::Debug,
    LogValueType: Serialize + DeserializeOwned + core::fmt::Debug,
{
    loop {
        if input_channel.is_abandoned() && input_channel.is_empty() {
            tracing::info!("Input channel is abandonded and empty, exiting");
            return Ok(()); // returning causes all of the client log message channels to be abandoned as well.
        }
        let num_to_read = input_channel.slots();
        let chunk = input_channel.read_chunk(num_to_read).unwrap();
        chunk
            .into_iter()
            .flat_map(|pkt| {
                let buf: PktMbuf<Vec<Message<LogKeyType, LogValueType>>> = PktMbuf::from(pkt);
                let data = buf.data_as_byte_slice();
                let msg: Vec<Message<LogKeyType, LogValueType>> = bincode::options()
                    .reject_trailing_bytes()
                    .with_fixint_encoding()
                    .with_native_endian()
                    .with_limit(10_000) // Larger than the packet size that should be accepted.
                    .deserialize(data)
                    .unwrap();
                msg
            })
            .for_each(|msg| {
                match msg {
                    Message::IncomingClientMessage(mut msg) => loop {
                        tracing::trace!(event = "Got message", msg = %msg);
                        let client_id = msg.client_id.0 as usize;
                        match client_log_message_channels[client_id].push(msg) {
                            Ok(()) => break,
                            Err(rtrb::PushError::Full(err)) => {
                                msg = err;
                            }
                        }
                    },
                    Message::AckMessage(_) => unimplemented!(), // This shouldn't happen
                }
            });
    }
}

/// Parallel when partitioned by client ID
#[instrument(skip(input_channel, out_channel, next_holder))]
pub fn order_messages<LogKeyType, LogValueType>(
    mut input_channel: SpscConsumerChannelHandle<ClientLogMessage<LogKeyType, LogValueType>>,
    mut out_channel: SpscProducerChannelHandle<ClientLogMessage<LogKeyType, LogValueType>>,
    next_holder: &AtomicU32,
    client_id: ClientId,
) -> Result<(), anyhow::Error>
where
    LogKeyType: Debug,
    LogValueType: Debug,
{
    let mut ordering_list: LinkedList<ClientLogMessage<LogKeyType, LogValueType>> =
        LinkedList::new();
    loop {
        if input_channel.is_abandoned() && input_channel.is_empty() && ordering_list.is_empty() {
            tracing::info!("Input channel abandoned and empty, closing");
            return Ok(()); // returning causes all of the client log message channels to be abandoned as well.
        }
        while out_channel.slots() == 0 {
            if input_channel.is_abandoned() && input_channel.is_empty() {
                tracing::info!("Input channel abandoned and empty, closing");
                return Ok(()); // returning causes all of the client log message channels to be abandoned as well.
            }
        }
        match input_channel.pop() {
            Ok(msg) => {
                let _msg_span =
                    tracing::trace_span!("msg_ordering", msg_id = msg.message_id.0).entered();
                tracing::trace!(event = "Got new message", msg = %msg);
                let mut next = next_holder.load(Ordering::Acquire);
                if msg.message_id.0 == next {
                    tracing::trace!(event = "Message found to be the next");
                    while out_channel.slots() == 0 {}
                    ordering_list.push_front(msg);

                    let mut num_to_split_off = ordering_list
                        .iter()
                        .take_while(|msg| {
                            if msg.message_id.0 == next {
                                next += 1;
                                next_holder.fetch_add(1, Ordering::Release);
                                true
                            } else {
                                false
                            }
                        })
                        .count();
                    if num_to_split_off > 0 {
                        tracing::trace!(
                            event = "Trying to send additional messages",
                            count = num_to_split_off
                        );
                        // for _ in 0..num_to_split_off {
                        //     let item = ordering_list.pop_front().expect("Should always be fine because num_to_split_off <= ordering_list.len()");
                        //     let mut res = Err(PushError::Full(item));
                        //     while res.is_err() {
                        //         match res.unwrap_err() {
                        //             PushError::Full(item) => {
                        //                 res = out_channel.push(item);
                        //             }
                        //         }
                        //     }
                        // }

                        let new_ordering_list = ordering_list.split_off(num_to_split_off);
                        let mut items_to_write = ordering_list;
                        ordering_list = new_ordering_list;

                        while num_to_split_off > 0 {
                            let num_to_write_this_round =
                                out_channel.slots().min(items_to_write.len());
                            let output_chunk = out_channel
                                .write_chunk_uninit(num_to_write_this_round)
                                .unwrap();
                            let remainder = items_to_write.split_off(num_to_write_this_round);
                            let num_written =
                                output_chunk.fill_from_iter(items_to_write.into_iter());
                            items_to_write = remainder;
                            num_to_split_off -= num_written;
                            if num_written > 0 {
                                tracing::trace!(
                                    event = "Sent messages",
                                    count = num_written,
                                    remaining = num_to_split_off
                                );
                            }
                        }
                        // next_holder.fetch_add(num_to_split_off as u32, Ordering::Release);
                    }
                } else {
                    // Unhappy path
                    let mut cursor = ordering_list.cursor_back_mut();
                    while let Some(cur) = cursor.current() && cur.message_id > msg.message_id {
                        cursor.move_prev()
                    }
                    cursor.insert_after(msg);
                    tracing::trace!(
                        event = "Message was not next, adding to buffer",
                        buffer_len = ordering_list.len()
                    );
                }
            }
            Err(_) => {
                if input_channel.is_abandoned() && input_channel.is_empty() {
                    tracing::info!("Input channel abandoned and empty, closing");
                    return Ok(());
                }
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
    let mut msg_id = 0;
    let msg_id_ref = &mut msg_id;
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
            for item in first.iter().chain(second) {
                assert_eq!(*msg_id_ref, item.message_id.0);
                *msg_id_ref += 1;
            }
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
        sync::{atomic::AtomicU32, Arc},
    };

    use hashbrown::hash_map::DefaultHashBuilder;

    use crate::{
        message::{
            ack::AckMessage,
            client_message::{ClientId, ClientLogMessage, MessageId},
        },
        prefix::Prefix,
        state::State,
        sync_provider::{test::TestLocalSyncProvider, SyncProvider},
        workers::pipeline::{
            apply_messages_pipeline, order_messages,
            test::ordering::get_sequence_of_n_shuffled_messages,
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
        use std::sync::{atomic::AtomicU32, Arc};

        use arbitrary::{Arbitrary, Unstructured};
        use itertools::Itertools;
        use rand::{seq::SliceRandom, Rng};

        use crate::{
            message::client_message::{ClientId, ClientLogMessage, MessageId},
            workers::pipeline::order_messages,
        };

        pub fn get_sequence_of_n_messages<
            LogKeyType: for<'a> Arbitrary<'a> + Clone,
            LogValueType: for<'a> Arbitrary<'a> + Clone,
            const NUM_MESSAGES: usize,
        >() -> Vec<ClientLogMessage<LogKeyType, LogValueType>> {
            let num_bytes =
                std::mem::size_of::<ClientLogMessage<LogKeyType, LogValueType>>() * NUM_MESSAGES;
            let mut rand = rand::thread_rng();
            let data = (0..num_bytes)
                .map(|_| rand.gen::<u64>())
                .flat_map(u64::to_ne_bytes)
                .collect_vec();
            let mut unstructured = Unstructured::new(&data);
            let mut msg_id = 0;
            let msgs: Box<[ClientLogMessage<LogKeyType, LogValueType>; NUM_MESSAGES]> =
                unstructured.arbitrary().unwrap();
            msgs.into_iter()
                .map(|mut msg| {
                    msg.client_id = ClientId(0);
                    msg.message_id = MessageId(msg_id);
                    msg_id += 1;
                    msg
                })
                .collect_vec()
        }

        pub fn get_sequence_of_n_shuffled_messages<
            LogKeyType: for<'a> Arbitrary<'a> + Clone,
            LogValueType: for<'a> Arbitrary<'a> + Clone,
            const NUM_MESSAGES: usize,
        >() -> Vec<ClientLogMessage<LogKeyType, LogValueType>> {
            let mut random = rand::thread_rng();
            let mut messages =
                get_sequence_of_n_messages::<LogKeyType, LogValueType, NUM_MESSAGES>();
            messages.shuffle(&mut random);
            messages
        }

        #[test]
        fn single_client_msg_reorder_test() {
            const NUM_MESSAGES: usize = 1_000;
            let (mut pipeline_write_handle, input_channel) = rtrb::RingBuffer::new(NUM_MESSAGES);
            let (output_channel, mut pipeline_read_handle) = rtrb::RingBuffer::new(NUM_MESSAGES);
            let next_holder = Arc::new(AtomicU32::new(0));
            let next_holder_ref = next_holder;
            let join_handle = std::thread::spawn(move || {
                order_messages(input_channel, output_channel, &next_holder_ref, ClientId(0))
            });
            let messages = get_sequence_of_n_shuffled_messages::<u32, u32, NUM_MESSAGES>();
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
            order_messages(order_input, order_output, &next_holder_ref, ClientId(0))
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
        let messages = get_sequence_of_n_shuffled_messages::<u32, u32, NUM_MESSAGES>();
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
        const NUM_MESSAGES: usize = 1_000;
        const NUM_ROUNDS: usize = 10;
        let (mut pipeline_write_handle, order_input) = rtrb::RingBuffer::new(NUM_MESSAGES);
        let (order_output, apply_input) = rtrb::RingBuffer::new(NUM_MESSAGES);
        let (apply_output, mut pipeline_read_handle) = rtrb::RingBuffer::new(NUM_MESSAGES);

        let (mut prefix_write_handle, apply_prefix_input) = rtrb::RingBuffer::new(NUM_MESSAGES);

        let next_holder_for_order_handle = Arc::new(AtomicU32::new(0));

        std::thread::scope(|s| {
            let order_join_handle = s.spawn(move || {
                order_messages(
                    order_input,
                    order_output,
                    &next_holder_for_order_handle,
                    ClientId(0),
                )
            });

            let apply_handle = s.spawn(move || {
                let mut client_channels = [apply_input];
                apply_messages_pipeline::<u32, u32, 0, 1, Global, DefaultHashBuilder>(
                    &mut client_channels,
                    apply_prefix_input,
                    apply_output,
                    Global::default(),
                )
            });

            let (prefix_input_handle, ticker_input) = crossbeam_channel::bounded(NUM_ROUNDS);

            let ticker = s.spawn(move || {
                let mut sync_provider = TestLocalSyncProvider {
                    channel: ticker_input,
                };

                for _ in 0..NUM_ROUNDS {
                    prefix_write_handle
                        .push(sync_provider.tick().unwrap())
                        .unwrap();
                }
            });

            let messages = get_sequence_of_n_shuffled_messages::<u32, u32, NUM_MESSAGES>();
            for round in 0..NUM_ROUNDS {
                assert_eq!(
                    0,
                    pipeline_write_handle.slots(),
                    "pipeline write handle not empty"
                );
                let chunk = pipeline_write_handle
                    .write_chunk_uninit(NUM_MESSAGES)
                    .unwrap();
                let num_written = chunk.fill_from_iter(messages.clone());
                assert_eq!(NUM_MESSAGES, num_written, "Not all messages were written.");
                prefix_input_handle
                    .send(Prefix {
                        id: round,
                        states: [MessageId((NUM_MESSAGES * (round + 1)) as u32)],
                    })
                    .unwrap();
                let mut to_get = NUM_MESSAGES;
                while to_get > 0 {
                    to_get -= pipeline_read_handle
                        .read_chunk(pipeline_read_handle.slots())
                        .into_iter()
                        .count();
                }
                assert_eq!(
                    0,
                    pipeline_read_handle.slots(),
                    "Not everything was ready from the pipeline"
                );
            }
            drop(pipeline_write_handle);
            order_join_handle.join().unwrap().unwrap();
            let _state = apply_handle.join().unwrap().unwrap();
            ticker.join().unwrap();
        });
    }
}
