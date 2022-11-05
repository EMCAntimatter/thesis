use std::hash::Hash;
use std::sync::Arc;
use std::sync::Barrier;

use itertools::Itertools;
use partitioned_swiss_table::table::PartitionedHashMap;
use rand::Rng;

use thesis::message::ack::AckMessage;
use thesis::message::client_message::{
    ClientId, ClientLogMessage, ClientMessageOperation, MessageId,
};

const NUM_MESSAGES: usize = 100_000;
const NUM_THREADS: usize = 8;
const NUM_ITERS: usize = 1_000;

type ChannelProducer<T> = cueue::Writer<T>;
type ChannelConsumer<T> = cueue::Reader<T>;

const MAX_CHUNK_SIZE: usize = 256;

fn new_state<K, V, const PARTITIONS: usize>(
) -> Arc<partitioned_swiss_table::table::PartitionedHashMap<K, V, PARTITIONS>>
where
    K: Eq + Hash,
{
    partitioned_swiss_table::table::PartitionedHashMap::with_capacity(1_000)
}

fn apply_messages<K, V, const PARTITIONS: usize>(
    state: &mut partitioned_swiss_table::table::PartitionedHashMapHandle<K, V, PARTITIONS>,
    input_channel: &mut ChannelConsumer<Option<(u64, ClientLogMessage<K, V>)>>,
    output_channel: &mut ChannelProducer<AckMessage<V>>,
) -> Result<(), ()>
where
    K: Eq + Hash,
    V: Clone,
    AckMessage<V>: Default,
{
    let mut return_flag: bool = false;
    loop {
        let buf = input_channel.read_chunk();
        let mut acks = Vec::with_capacity(buf.len());
        for v in buf {
            match v {
                Some((hash, msg)) => {
                    let ack = match &msg.operation {
                        ClientMessageOperation::Get { key: _ } => {
                            let result = state.get(*hash).cloned();
                            AckMessage {
                                client_id: msg.client_id,
                                message_id: msg.message_id,
                                extension: thesis::message::ack::AckMessageExtensions::Get(result),
                            }
                        }
                        ClientMessageOperation::Put { key: _, value } => {
                            let prev = state.put(*hash, value.clone());
                            AckMessage {
                                client_id: msg.client_id,
                                message_id: msg.message_id,
                                extension: thesis::message::ack::AckMessageExtensions::Put(prev),
                            }
                        }
                        ClientMessageOperation::Delete { key: _ } => {
                            let removed = state.delete(*hash);
                            AckMessage {
                                client_id: msg.client_id,
                                message_id: msg.message_id,
                                extension: thesis::message::ack::AckMessageExtensions::Delete(
                                    removed,
                                ),
                            }
                        }
                    };
                    acks.push(ack);
                }
                None => {
                    return_flag = true;
                }
            }
        }
        input_channel.commit();
        let mut slice_to_copy_over: &[_] = &acks;
        loop {
            let write_end = output_channel.write_chunk();
            let num_to_copy = slice_to_copy_over.len().min(write_end.len());
            // memcpy into buffer
            unsafe {
                std::ptr::copy_nonoverlapping(acks.as_ptr(), write_end.as_mut_ptr(), num_to_copy);
            }
            output_channel.commit(num_to_copy);
            if num_to_copy >= slice_to_copy_over.len() {
                break;
            }
            slice_to_copy_over = &slice_to_copy_over[(num_to_copy - 1)..slice_to_copy_over.len()];
        }
        if input_channel.is_abandoned() {
            return Err(());
        } else if return_flag {
            return Ok(());
        }
    }
}

pub fn main() {
    let msg = ClientLogMessage {
        client_id: ClientId(0),
        message_id: MessageId(0),
        operation: ClientMessageOperation::Put {
            key: [0u8; 128],
            value: [0u8; 128],
        },
    };
    let state = new_state::<[u8; 128], [u8; 128], NUM_THREADS>();
    let mut handles = PartitionedHashMap::create_all_handles(&state);

    let mut input_channels = (0..NUM_THREADS).map(|_| cueue::cueue(NUM_MESSAGES).unwrap());
    let start_barrier = Arc::new(Barrier::new(NUM_THREADS + 1));
    let end_barrier = Arc::new(Barrier::new(NUM_THREADS + 1));
    let mut random = rand::thread_rng();
    let messages = (0..NUM_MESSAGES).map(|_| {
        let mut msg = msg;
        match &mut msg.operation {
            ClientMessageOperation::Put { key, value } => {
                random.fill(key);
                random.fill(value);
            }
            ClientMessageOperation::Get { key } => {
                random.fill(key);
            }
            ClientMessageOperation::Delete { key } => {
                random.fill(key);
            }
        }
        msg
    });

    let mut output_channels: [Option<ChannelConsumer<_>>; NUM_THREADS] =
        std::array::from_fn(|_| None);

    let mut send_channels: [ChannelProducer<_>; NUM_THREADS] = std::array::from_fn(|i| {
        let t_start_barrier = start_barrier.clone();
        let t_end_barrier = end_barrier.clone();
        let (send, recv) = input_channels.next().unwrap();
        let handle = handles[i].take().unwrap();
        let (t_output_send, t_output_recv) =
            cueue::cueue::<AckMessage<[u8; 128]>>(NUM_MESSAGES).unwrap();
        output_channels[i].replace(t_output_recv);

        let t_builder = std::thread::Builder::new();
        t_builder
            .name(format!("partition {i} handler"))
            .spawn(move || {
                // let mut count = 0;
                let mut handle_local = handle;
                let mut recv_local = recv;
                let mut output_local = t_output_send;
                loop {
                    t_start_barrier.wait();
                    if let Err(()) =
                        apply_messages(&mut handle_local, &mut recv_local, &mut output_local)
                    {
                        return;
                    }
                    t_end_barrier.wait();
                    // println!("{i},{count},{}", handle_local.len());
                    handle_local.clear();
                    // count += 1;
                }
            })
            .unwrap();
        send
    });

    let mut output_channels = output_channels.map(Option::unwrap);

    let messages = messages
        .map(|msg| {
            let (partition, hash) = state.get_partition_and_key_hash(msg.operation.get_key());
            (partition, hash, msg)
        })
        .collect_vec();

    let mut durations: Box<[i64; NUM_ITERS]> = Box::new(std::array::from_fn(|_| 0));

    for i in 0..NUM_ITERS {
        for (partition, hash, msg) in messages.iter() {
            let mut result: Result<(), Option<(u64, _)>> = Err(Some((*hash, *msg)));
            while let Err(Some((hash, msg))) = result {
                let channel_send_result = send_channels[*partition].push(Some((hash, msg)));
                result = channel_send_result;
            }
        }
        for chan in &mut send_channels {
            chan.push(None).unwrap();
        }
        start_barrier.wait();
        let start = chrono::Local::now();
        end_barrier.wait();
        let end = chrono::Local::now();
        let duration = end - start;
        // println!("{}", duration);
        durations[i] = duration.num_nanoseconds().unwrap();
        let mut total_acks = 0;
        for chan in output_channels.iter_mut() {
            let buf = chan.read_chunk();
            total_acks += buf.len();
            chan.commit();
        }
        assert_eq!(NUM_MESSAGES, total_acks);
    }
    let sum: i64 = durations.into_iter().sum();
    let average: i64 = sum / NUM_ITERS as i64;

    println!("Average: {} ns", average)
}
