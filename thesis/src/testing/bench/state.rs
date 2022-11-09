use std::fmt::Debug;

use crate::{
    message::client_message::ClientLogMessage, state::State,
    workers::pipeline::apply_message_to_state,
};

const NUM_MESSAGES: usize = 10_000;
const NUM_THREADS: usize = 12;

#[inline]
fn apply_message_to_state_n_times<LogKeyType, LogValueType>(
    msg: ClientLogMessage<LogKeyType, LogValueType>,
    state: &mut impl State<LogKeyType, LogValueType>,
) where
    LogKeyType: Clone + Debug,
    LogValueType: Clone + Debug,
{
    for _ in 0..NUM_MESSAGES {
        let _ack = apply_message_to_state(msg.clone(), state);
    }
}

mod hashtable {
    mod hashbrown {

        use std::alloc::Global;
        use std::hash::Hash;

        use rand::Rng;
        use test::Bencher;

        use crate::{
            message::client_message::{
                ClientId, ClientLogMessage, ClientMessageOperation, MessageId,
            },
            state::State,
            testing::bench::state::{apply_message_to_state_n_times, NUM_MESSAGES},
            workers::pipeline::apply_message_to_state,
        };

        fn new_state<K, V>() -> impl State<K, V, Global>
        where
            K: Eq + Hash,
        {
            hashbrown::HashMap::with_capacity(NUM_MESSAGES)
        }

        #[bench]
        fn single_key_write(b: &mut Bencher) {
            let msg = ClientLogMessage {
                client_id: ClientId(0),
                message_id: MessageId(0),
                operation: crate::message::client_message::ClientMessageOperation::Put {
                    key: [0u8; 128],
                    value: [0u8; 128],
                },
            };
            let mut state = new_state();
            b.iter(|| {
                apply_message_to_state_n_times(msg, &mut state);
            })
        }

        #[bench]
        fn single_key_read(b: &mut Bencher) {
            let msg = ClientLogMessage {
                client_id: ClientId(0),
                message_id: MessageId(0),
                operation: ClientMessageOperation::Get { key: [0u8; 128] },
            };
            let mut state = new_state();
            state.put([0u8; 128], [0u8; 128]);
            b.iter(|| {
                apply_message_to_state_n_times(msg, &mut state);
            })
        }

        #[bench]
        fn random_key_write(b: &mut Bencher) {
            // tracing_subscriber::fmt::init();
            let msg = ClientLogMessage {
                client_id: ClientId(0),
                message_id: MessageId(0),
                operation: ClientMessageOperation::Put {
                    key: [0u8; 128],
                    value: [0u8; 128],
                },
            };
            let mut state = new_state();
            let mut random = rand::thread_rng();
            let messages: Vec<_> = (0..NUM_MESSAGES)
                .map(|_| {
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
                })
                .collect();
            b.iter(|| {
                let msgs = messages.clone();
                for msg in msgs.into_iter() {
                    let _ack = apply_message_to_state(msg, &mut state);
                }
                state.clear()
            })
        }

        #[bench]
        fn random_key_read(b: &mut Bencher) {
            let msg = ClientLogMessage {
                client_id: ClientId(0),
                message_id: MessageId(0),
                operation: ClientMessageOperation::Get { key: [0u8; 128] },
            };
            let mut state = new_state();
            state.put([0u8; 128], [0u8; 128]);
            let mut random = rand::thread_rng();
            let messages: Vec<_> = (0..NUM_MESSAGES)
                .map(|_| {
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
                })
                .collect();
            b.iter(|| {
                for msg in messages.iter() {
                    let _ack = apply_message_to_state(*msg, &mut state);
                }
            })
        }
    }

    mod dashmap {

        use std::hash::Hash;
        use std::sync::Arc;
        use std::sync::Barrier;

        use rand::Rng;
        use test::Bencher;

        use crate::message::{
            ack::AckMessage,
            client_message::{ClientId, ClientLogMessage, ClientMessageOperation, MessageId},
        };
        use crate::testing::bench::state::NUM_MESSAGES;
        use crate::testing::bench::state::NUM_THREADS;

        use itertools::*;

        fn new_state<K, V>() -> dashmap::DashMap<K, V, ahash::RandomState>
        where
            K: Eq + Hash,
        {
            dashmap::DashMap::with_capacity_and_hasher(NUM_MESSAGES, ahash::RandomState::default())
        }

        #[bench]
        fn single_key_write(b: &mut Bencher) {
            let msg = ClientLogMessage {
                client_id: ClientId(0),
                message_id: MessageId(0),
                operation: crate::message::client_message::ClientMessageOperation::Put {
                    key: [0u8; 128],
                    value: [0u8; 128],
                },
            };
            let state = new_state();
            state.insert([0u8; 128], [0u8; 128]);
            b.iter(|| {
                for _ in 0..NUM_MESSAGES {
                    let _ack = {
                        let msg = msg;
                        match msg.operation {
                            ClientMessageOperation::Get { key } => {
                                let result = state.get(&key).map(|r| *r.value());
                                AckMessage {
                                    client_id: msg.client_id,
                                    message_id: msg.message_id,
                                    extension: crate::message::ack::AckMessageExtensions::Get(
                                        result,
                                    ),
                                }
                            }
                            ClientMessageOperation::Put { key, value } => {
                                let prev = state.insert(key, value);
                                AckMessage {
                                    client_id: msg.client_id,
                                    message_id: msg.message_id,
                                    extension: crate::message::ack::AckMessageExtensions::Put(prev),
                                }
                            }
                            ClientMessageOperation::Delete { key } => {
                                let removed = state.remove(&key).map(|(_, v)| v);
                                AckMessage {
                                    client_id: msg.client_id,
                                    message_id: msg.message_id,
                                    extension: crate::message::ack::AckMessageExtensions::Delete(
                                        removed,
                                    ),
                                }
                            }
                        }
                    };
                }
            })
        }

        #[bench]
        fn single_key_read(b: &mut Bencher) {
            let msg = ClientLogMessage {
                client_id: ClientId(0),
                message_id: MessageId(0),
                operation: ClientMessageOperation::Get { key: [0u8; 128] },
            };
            let state = new_state();
            state.insert([0u8; 128], [0u8; 128]);
            b.iter(|| {
                let _ack = {
                    let msg = msg;
                    match msg.operation {
                        ClientMessageOperation::Get { key } => {
                            let result = state.get(&key).map(|r| *r.value());
                            AckMessage {
                                client_id: msg.client_id,
                                message_id: msg.message_id,
                                extension: crate::message::ack::AckMessageExtensions::Get(result),
                            }
                        }
                        ClientMessageOperation::Put { key, value } => {
                            let prev = state.insert(key, value);
                            AckMessage {
                                client_id: msg.client_id,
                                message_id: msg.message_id,
                                extension: crate::message::ack::AckMessageExtensions::Put(prev),
                            }
                        }
                        ClientMessageOperation::Delete { key } => {
                            let removed = state.remove(&key).map(|(_, v)| v);
                            AckMessage {
                                client_id: msg.client_id,
                                message_id: msg.message_id,
                                extension: crate::message::ack::AckMessageExtensions::Delete(
                                    removed,
                                ),
                            }
                        }
                    }
                };
            })
        }

        #[bench]
        fn random_key_write(b: &mut Bencher) {
            let msg = ClientLogMessage {
                client_id: ClientId(0),
                message_id: MessageId(0),
                operation: ClientMessageOperation::Put {
                    key: [0u8; 128],
                    value: [0u8; 128],
                },
            };
            let state = new_state();
            state.insert([0u8; 128], [0u8; 128]);
            let mut random = rand::thread_rng();
            let messages: Vec<_> = (0..NUM_MESSAGES)
                .map(|_| {
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
                })
                .collect();
            b.iter(|| {
                for msg in messages.iter() {
                    let _ack = match msg.operation {
                        ClientMessageOperation::Get { key } => {
                            let result = state.get(&key).map(|r| *r.value());
                            AckMessage {
                                client_id: msg.client_id,
                                message_id: msg.message_id,
                                extension: crate::message::ack::AckMessageExtensions::Get(result),
                            }
                        }
                        ClientMessageOperation::Put { key, value } => {
                            let prev = state.insert(key, value);
                            AckMessage {
                                client_id: msg.client_id,
                                message_id: msg.message_id,
                                extension: crate::message::ack::AckMessageExtensions::Put(prev),
                            }
                        }
                        ClientMessageOperation::Delete { key } => {
                            let removed = state.remove(&key).map(|(_, v)| v);
                            AckMessage {
                                client_id: msg.client_id,
                                message_id: msg.message_id,
                                extension: crate::message::ack::AckMessageExtensions::Delete(
                                    removed,
                                ),
                            }
                        }
                    };
                }
                state.clear()
            })
        }

        fn apply_messages_after_barrier(
            state: Arc<dashmap::DashMap<[u8; 128], [u8; 128], ahash::RandomState>>,
            start_barrier: Arc<Barrier>,
            end_barrier: Arc<Barrier>,
            messages: Vec<ClientLogMessage<[u8; 128], [u8; 128]>>,
        ) {
            start_barrier.wait();
            for msg in messages.into_iter() {
                let _ack = match msg.operation {
                    ClientMessageOperation::Get { key } => {
                        let result = state.get(&key).map(|r| *r.value());
                        AckMessage {
                            client_id: msg.client_id,
                            message_id: msg.message_id,
                            extension: crate::message::ack::AckMessageExtensions::Get(result),
                        }
                    }
                    ClientMessageOperation::Put { key, value } => {
                        let prev = state.insert(key, value);
                        AckMessage {
                            client_id: msg.client_id,
                            message_id: msg.message_id,
                            extension: crate::message::ack::AckMessageExtensions::Put(prev),
                        }
                    }
                    ClientMessageOperation::Delete { key } => {
                        let removed = state.remove(&key).map(|(_, v)| v);
                        AckMessage {
                            client_id: msg.client_id,
                            message_id: msg.message_id,
                            extension: crate::message::ack::AckMessageExtensions::Delete(removed),
                        }
                    }
                };
            }
            end_barrier.wait();
        }

        #[bench]
        fn random_key_write_mt(b: &mut Bencher) {
            let msg = ClientLogMessage {
                client_id: ClientId(0),
                message_id: MessageId(0),
                operation: ClientMessageOperation::Put {
                    key: [0u8; 128],
                    value: [0u8; 128],
                },
            };
            let state = new_state();
            state.insert([0u8; 128], [0u8; 128]);
            let state = Arc::new(state);
            let start_barrier = Arc::new(Barrier::new(NUM_THREADS));
            let end_barrier = Arc::new(Barrier::new(NUM_THREADS));
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

            let messages = messages.chunks((NUM_MESSAGES / NUM_THREADS) + 1);
            let mut messages = messages.into_iter();

            for _ in 0..(NUM_THREADS - 1) {
                let t_state_ref = state.clone();
                let t_start_barrier = start_barrier.clone();
                let t_end_barrier = end_barrier.clone();
                let messages_t = messages.next().unwrap().collect_vec();
                std::thread::spawn(move || loop {
                    apply_messages_after_barrier(
                        t_state_ref.clone(),
                        t_start_barrier.clone(),
                        t_end_barrier.clone(),
                        messages_t.clone(),
                    )
                });
            }

            let messages = messages.next().unwrap().collect_vec();

            b.iter(|| {
                apply_messages_after_barrier(
                    state.clone(),
                    start_barrier.clone(),
                    end_barrier.clone(),
                    messages.clone(),
                );
                state.clear();
            })
        }

        #[bench]
        fn random_key_read(b: &mut Bencher) {
            let msg = ClientLogMessage {
                client_id: ClientId(0),
                message_id: MessageId(0),
                operation: ClientMessageOperation::Get { key: [0u8; 128] },
            };
            let state = new_state();
            state.insert([0u8; 128], [0u8; 128]);
            let mut random = rand::thread_rng();
            let messages: Vec<_> = (0..NUM_MESSAGES)
                .map(|_| {
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
                })
                .collect();
            b.iter(|| {
                for msg in messages.iter() {
                    let msg = *msg;
                    let _ack: AckMessage<[u8; 128]> = match msg.operation {
                        ClientMessageOperation::Get { key } => {
                            let result = state.get(&key).map(|r| *r.value());
                            AckMessage {
                                client_id: msg.client_id,
                                message_id: msg.message_id,
                                extension: crate::message::ack::AckMessageExtensions::Get(result),
                            }
                        }
                        ClientMessageOperation::Put { key, value } => {
                            let prev = state.insert(key, value);
                            AckMessage {
                                client_id: msg.client_id,
                                message_id: msg.message_id,
                                extension: crate::message::ack::AckMessageExtensions::Put(prev),
                            }
                        }
                        ClientMessageOperation::Delete { key } => {
                            let removed = state.remove(&key).map(|(_, v)| v);
                            AckMessage {
                                client_id: msg.client_id,
                                message_id: msg.message_id,
                                extension: crate::message::ack::AckMessageExtensions::Delete(
                                    removed,
                                ),
                            }
                        }
                    };
                }
            })
        }
    }
}

#[cfg(not(bench))]
mod btree {
    use std::{alloc::Global, collections::BTreeMap};

    use rand::Rng;
    use test::Bencher;

    use crate::{
        message::client_message::{ClientId, ClientLogMessage, ClientMessageOperation, MessageId},
        state::State,
        workers::pipeline::apply_message_to_state,
    };

    use super::{apply_message_to_state_n_times, NUM_MESSAGES};

    fn new_state<K, V>() -> impl State<K, V, Global>
    where
        K: Eq + Ord,
    {
        BTreeMap::new()
    }

    #[bench]
    fn single_key_write(b: &mut Bencher) {
        tracing_subscriber::fmt::init();
        let msg = ClientLogMessage {
            client_id: ClientId(0),
            message_id: MessageId(0),
            operation: crate::message::client_message::ClientMessageOperation::Put {
                key: *b"I_need_something_64_bytes_long_so_this_is_some_test_data_foo_bar",
                value: 0u32,
            },
        };
        let mut state = hashbrown::HashMap::new();
        b.iter(|| {
            apply_message_to_state_n_times(msg, &mut state);
            state.clear();
        })
    }

    #[bench]
    fn single_key_read(b: &mut Bencher) {
        tracing_subscriber::fmt::init();

        let msg = ClientLogMessage {
            client_id: ClientId(0),
            message_id: MessageId(0),
            operation: ClientMessageOperation::Get {
                key: *b"I_need_something_64_bytes_long_so_this_is_some_test_data_foo_bar",
            },
        };
        let mut state = new_state();
        state.put(
            *b"I_need_something_64_bytes_long_so_this_is_some_test_data_foo_bar",
            1,
        );
        b.iter(|| {
            apply_message_to_state_n_times(msg, &mut state);
            state.clear();
        })
    }

    #[bench]
    fn random_key_write(b: &mut Bencher) {
        tracing_subscriber::fmt::init();

        let mut msg = ClientLogMessage {
            client_id: ClientId(0),
            message_id: MessageId(0),
            operation: ClientMessageOperation::Put {
                key: [0u8; 128],
                value: [0u8; 128],
            },
        };
        let mut state = new_state();
        let mut random = rand::thread_rng();
        b.iter(|| {
            for _ in 0..NUM_MESSAGES {
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
                let _ack = apply_message_to_state(msg, &mut state);
            }
            state.clear()
        })
    }

    #[bench]
    fn random_key_read(b: &mut Bencher) {
        tracing_subscriber::fmt::init();

        let mut msg = ClientLogMessage {
            client_id: ClientId(0),
            message_id: MessageId(0),
            operation: ClientMessageOperation::Get { key: [0u8; 128] },
        };
        let mut state = new_state();
        state.put([0u8; 128], [0u8; 128]);
        let mut random = rand::thread_rng();
        b.iter(|| {
            for _ in 0..NUM_MESSAGES {
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
                let _ack = apply_message_to_state(msg, &mut state);
            }
        })
    }
}
