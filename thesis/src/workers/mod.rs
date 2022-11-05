pub mod eth;
pub mod pipeline;

use dpdk::{
    self,
    device::event::event_interface::{dequeue_events, enqueue_new_events},
    eal::current_lcore_id,
    raw::{rte_event, rte_mbuf, RTE_EVENT_OP_RELEASE},
};

use libc::{c_int, c_void};
use parking_lot::{Mutex, Once};

use std::{
    mem::MaybeUninit,
    sync::atomic::{fence, Ordering},
};

use crate::{
    EVENTDEV_DEVICE_ID, EVENT_HANDLER_PORT_ID, RUNNING, TERMINATE, TX_ADAPTER_INPUT_QUEUE_ID,
};

use self::{
    eth::{read_packets_from_nic_port_0_into_channel, write_packets_to_nic_port_0},
    pipeline::{SpscConsumerChannelHandle, SpscProducerChannelHandle},
};

const RX_RING_BUFFER_SIZE: usize = 1024;

// type RxRingBufferType<'a> = RingBufferInterface<RX_RING_BUFFER_SIZE, &'a mut rte_mbuf>;
// type TxRingBufferType<'a> = RingBufferInterface<RX_RING_BUFFER_SIZE, &'a mut rte_mbuf>;

// pub(crate) const RX_RING_BUFFER: OnceCell<RxRingBufferType> = OnceCell::new();
// pub(crate) const TX_RING_BUFFER: OnceCell<TxRingBufferType> = OnceCell::new();

// fn buffer_init_helper() {
//     RX_RING_BUFFER.get_or_init(|| RingBuffer::new());
//     TX_RING_BUFFER.get_or_init(|| RingBuffer::new());
// }

const BUFFER_SIZE: usize = 1024;
static IO_GUARD: Once = Once::new();
static INPUT: Mutex<Option<SpscProducerChannelHandle<&mut rte_mbuf>>> = Mutex::new(None);
static OUTPUT: Mutex<Option<SpscConsumerChannelHandle<&mut rte_mbuf>>> = Mutex::new(None);

fn channel_init_helper() {
    IO_GUARD.call_once(|| {
        let (send, recv) = rtrb::RingBuffer::new(BUFFER_SIZE);
        let mut input_lock = INPUT.lock();
        let mut output_lock = OUTPUT.lock();
        input_lock.replace(send);
        output_lock.replace(recv);
    });
}

pub extern "C" fn lcore_init(_unused: *mut c_void) -> c_int {
    // buffer_init_helper();
    channel_init_helper();
    fence(Ordering::SeqCst);
    let lcore = current_lcore_id();
    match lcore {
        // 0 | 1 => {}
        2 => {
            read_packets_from_nic_port_0_into_channel::<RX_RING_BUFFER_SIZE>(
                INPUT.lock().take().unwrap(),
            );
        }
        3 => {
            write_packets_to_nic_port_0(OUTPUT.lock().take().unwrap()).unwrap();
        }
        // 4 => handle_events(),
        _ => {
            println!("Extra lcore {lcore} exiting");
            return 0;
        }
    }

    0
}

#[allow(dead_code)]
fn handle_events() {
    RUNNING.with_guard_1(|| {
        let lcore = current_lcore_id();
        println!("Lcore {lcore} handling events");

        // spin_on_flag!(SERVICE_SETUP_DONE, "Event Handler");

        let mut event_buf: [rte_event; 32] = [unsafe { MaybeUninit::zeroed().assume_init() }; 32];

        let mut recv_counter: u64 = 1;
        let mut send_counter: u64 = 1;

        loop {
            let length = dequeue_events(
                EVENTDEV_DEVICE_ID,
                EVENT_HANDLER_PORT_ID,
                &mut event_buf,
                100,
            );
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
