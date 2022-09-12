use crate::{event_handlers::{setup_rx_adapter_worker, setup_tx_adapter_worker}, ETHDEV_PORT_ID, TX_ADAPTER_INPUT_QUEUE_ID};
use anyhow::Context;
use dpdk::{
    self,
    device::event::{
        eth::rx::rx_adapter::{RxAdapterQueueConfig, RxAdapterQueueConfigExtensions},
        event::{self, Event},
        event_interface::{dequeue_events, enqueue_new_events},
    },
    eal::current_lcore_id,
    raw::{rte_event, RTE_EVENT_OP_RELEASE},
};
use libc::{c_int, c_void};

use std::mem::MaybeUninit;

use crate::{EVENTDEV_DEVICE_ID, EVENT_HANDLER_PORT_ID, RUNNING, SERVICE_SETUP_DONE, TERMINATE};

macro_rules! spin_on_flag {
    ($flag_name:ident, $name:expr) => {
        println!("{} is waiting on {}", $name, stringify!($flag_name));
        while !$flag_name.load(std::sync::atomic::Ordering::Acquire) {}
        println!(
            "{} is no longer waiting on {}",
            $name,
            stringify!($flag_name)
        );
    };
}


pub extern "C" fn lcore_init(_unused: *mut c_void) -> c_int {
    let lcore = current_lcore_id();
    match lcore {
        // 0 | 1 => {}
        // 2 => inject_events(),
        // 6 => tx_adapter_worker(),
        // 7 => rx_adapter_worker(),
        3 => handle_events(),
        _ => println!("Extra lcore {lcore} exiting"),
    }

    return 0;
}


fn handle_events() {
    RUNNING.with_guard_1(|| {
        let lcore = current_lcore_id();
        println!("Lcore {lcore} handling events");

        // spin_on_flag!(SERVICE_SETUP_DONE, "Event Handler");

        let mut event_buf: [rte_event; 32] = [unsafe { MaybeUninit::zeroed().assume_init() }; 32];

        let mut recv_counter: u64 = 1;
        let mut send_counter: u64 = 1;

        loop {
            let length =
                dequeue_events(EVENTDEV_DEVICE_ID, EVENT_HANDLER_PORT_ID, &mut event_buf, 100);
            recv_counter += length as u64;
            if length > 0 {
                println!("dequeued: {length}, {recv_counter}");
                for i in 0..length {
                    let event: &mut rte_event = &mut event_buf[i as usize];
                    unsafe {
                        let event_config_data = &mut event.__bindgen_anon_1.__bindgen_anon_1;
                        event_config_data.set_op(RTE_EVENT_OP_RELEASE as u8);
                        event_config_data.queue_id = TX_ADAPTER_INPUT_QUEUE_ID as u8;
                        // event_config_data.queue_id = 0;
                    }
                }
                let fowarded = enqueue_new_events(
                    EVENTDEV_DEVICE_ID,
                    EVENT_HANDLER_PORT_ID,
                    &mut event_buf,
                    length as usize,
                );
                if fowarded > 0 {
                    send_counter += fowarded as u64;
                    println!("fowarded: {fowarded}, {send_counter}");
                }
            }
            if TERMINATE.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
        }
    })
}

pub fn rx_adapter_worker() {
    RUNNING.with_guard_1(|| {
        let queue_config = RxAdapterQueueConfig {
            rx_queue_flags: 0,
            servicing_weight: 1,
            event: Event {
                word0: event::EventWordZero::EventAttributes {
                    queue_id: 0,
                    priority: 0,
                    scheduling_type: event::EventScheduleType::Parallel,
                    flow_id: 0,
                    event_type: event::EventType::EthRxAdapter,
                    event_subtype: 0,
                    operation: event::EventOp::New,
                },
                word1: Default::default(),
            },
            event_buf_size: 0,
            extensions: RxAdapterQueueConfigExtensions::None,
        };
        println!(
            "Setting up RX adapter worker on core {}",
            current_lcore_id()
        );
        setup_rx_adapter_worker(0, EVENTDEV_DEVICE_ID, ETHDEV_PORT_ID, queue_config, 0).unwrap();
        println!(
            "Finished Setting up RX adapter worker on core {}",
            current_lcore_id()
        );

        SERVICE_SETUP_DONE.store(true, std::sync::atomic::Ordering::SeqCst);
    })
}

pub fn tx_adapter_worker() {
    RUNNING.with_guard_1(|| {
        setup_tx_adapter_worker(0, EVENTDEV_DEVICE_ID, ETHDEV_PORT_ID, -1)
            .context(format!("Setting up tx adapter 0"))
            .unwrap();
    })
}