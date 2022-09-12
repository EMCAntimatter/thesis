use std::{collections::LinkedList, sync::atomic::AtomicUsize};

use dpdk::{device::{eth::dev::EventQueueId, event::event_interface::dequeue_events}, raw::rte_event};

use crate::EVENTDEV_DEVICE_ID;

use super::LogMessage;

pub const MAX_CLIENTS: u32 = 4;
pub const CLIENT_MANAGER_POOL_SIZE: u32 = 1;
pub const CLIENT_MANAGER_QUEUE_ID_START: EventQueueId = 0;

pub type ClientLogManagerStorage = LinkedList<LogMessage>;

static CLIENT_LOG_MANAGER_ID_CREATOR: AtomicUsize = AtomicUsize::new(0);

pub type ClientLogManagerId = usize;

pub struct ClientLogManager {
    id: ClientLogManagerId,
    queue_id: EventQueueId,
    event_storage: [rte_event; 32],
    storage: [ClientLogManagerStorage; (MAX_CLIENTS / CLIENT_MANAGER_POOL_SIZE) as usize],
}

impl ClientLogManager {
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
        let length = dequeue_events(EVENTDEV_DEVICE_ID, self.queue_id as u8, &mut self.event_storage, 100) as usize;
        for i in 0..length {
            let event = self.event_storage[i];
            println!("{event:#?}")
        }
    }
}