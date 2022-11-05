use std::{collections::LinkedList, sync::atomic::AtomicUsize};

use dpdk::{
    device::{eth::dev::EventQueueId, event::event_interface::dequeue_events},
    raw::rte_event,
};

use crate::{message::client_message::ClientLogMessage, EVENTDEV_DEVICE_ID};

pub const MAX_CLIENTS: u32 = 4;
pub const CLIENT_MANAGER_POOL_SIZE: u32 = 1;
pub const CLIENT_MANAGER_QUEUE_ID_START: EventQueueId = 0;

pub type ClientLogManagerStorage<LogKeyType, LogValueType> =
    LinkedList<ClientLogMessage<LogKeyType, LogValueType>>;

static CLIENT_LOG_MANAGER_ID_CREATOR: AtomicUsize = AtomicUsize::new(0);

pub type ClientLogManagerId = usize;

pub struct ClientLogManager<LogKeyType, LogValueType> {
    id: ClientLogManagerId,
    queue_id: EventQueueId,
    event_storage: [rte_event; 32],
    storage: [ClientLogManagerStorage<LogKeyType, LogValueType>;
        (MAX_CLIENTS / CLIENT_MANAGER_POOL_SIZE) as usize],
}

impl<LogKeyType, LogValueType> ClientLogManager<LogKeyType, LogValueType> {
    pub fn new() -> Self {
        let id = CLIENT_LOG_MANAGER_ID_CREATOR.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Self {
            storage: Default::default(),
            id,
            queue_id: CLIENT_MANAGER_QUEUE_ID_START + (id as EventQueueId),
            event_storage: unsafe { std::mem::MaybeUninit::zeroed().assume_init() },
        }
    }

    pub fn get_events(&mut self) {
        let length = dequeue_events(
            EVENTDEV_DEVICE_ID,
            self.queue_id as u8,
            &mut self.event_storage,
            100,
        ) as usize;
        for i in 0..length {
            let _event = self.event_storage[i];
            // println!("{event:#?}")
        }
    }
}

impl<LogKeyType, LogValueType> Default for ClientLogManager<LogKeyType, LogValueType> {
    fn default() -> Self {
        Self::new()
    }
}
