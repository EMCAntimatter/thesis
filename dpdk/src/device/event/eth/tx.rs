use dpdk_sys::rte_event_eth_tx_adapter_caps_get;

use crate::device::{eth::dev::EthdevPortId, event::EventDeviceId};
use bitflags::bitflags;

bitflags! {
    #[repr(C)]
    pub struct EventDevTxCapabilities: u32 {
        /// Event scheduling prioritization is based on the priority associated with each event queue.
        const QUEUE_QOS_SCHEDULING = 0b0001;
        /// Event scheduling prioritization is based on the priority associated with
        /// each event. Priority of each event is supplied in *rte_event* structure
        /// on each enqueue operation.
        const EVENT_QOS_SCHEDULING = 0b0010;
        /// Event device operates in distributed scheduling mode.
        /// In distributed scheduling mode, event scheduling happens in HW or
        /// rte_event_dequeue_burst() or the combination of these two.
        /// If the flag is not set then eventdev is centralized and thus needs a
        /// dedicated service core that acts as a scheduling thread .
        const DISTRIBUTED_SCHEDULING = 0b0100;
        /// Event device is capable of enqueuing events of any type to any queue.
        /// If this capability is not set, the queue only supports events of the
        ///  `RTE_SCHED_TYPE_` type that it was created with.
        const QUEUE_ALL_EVENT_TYPES = 0b1000;
        /// Event device is capable of operating in burst mode for enqueue(forward,
        /// release) and dequeue operation. If this capability is not set, application
        /// still uses the rte_event_dequeue_burst() and rte_event_enqueue_burst() but
        /// PMD accepts only one event at a time.
        const EVENT_DEV_BURST_MODE = 0b10000;
        /// Event device ports support disabling the implicit release feature, in
        /// which the port will release all unreleased events in its dequeue operation.
        /// If this capability is set and the port is configured with implicit release
        /// disabled, the application is responsible for explicitly releasing events
        /// using either the RTE_EVENT_OP_FORWARD or the RTE_EVENT_OP_RELEASE event
        /// enqueue operations.
        const DISABLE_IMPLICIT_RELEASE = 0b100000;
        // Event device is capable of operating in none sequential mode. The path
        // of the event is not necessary to be sequential. Application can change
        // the path of event at runtime. If the flag is not set, then event each event
        // will follow a path from queue 0 to queue 1 to queue 2 etc. If the flag is
        // set, events may be sent to queues in any order. If the flag is not set, the
        // eventdev will return an error when the application enqueues an event for a
        // qid which is not the next in the sequence.
        const NONSEQUENTIAL_MODE =  0b1000000;
        // Event device is capable of configuring the queue/port link at runtime.
        // If the flag is not set, the eventdev queue/port link is only can be
        // configured during  initialization.
        const RUNTIME_PORT_LINK_CONFIG =  0b10000000;
        /// Event device is capable of setting up the link between multiple queue
        /// with single port. If the flag is not set, the eventdev can only map a
        /// single queue to each port or map a single queue to many port.
        const MULTI_QUEUE_TO_PORT =  0b100000000;
        /// Event device preserves the flow ID from the enqueued
        /// event to the dequeued event if the flag is set. Otherwise,
        /// the content of this field is implementation dependent.
        const CARRY_FLOW_ID =  0b1000000000;
        /// Event device *does not* require calls to rte_event_maintain().
        /// An event device that does not set this flag requires calls to
        /// rte_event_maintain() during periods when neither
        /// rte_event_dequeue_burst() nor rte_event_enqueue_burst() are called
        /// on a port. This will allow the event device to perform internal
        /// processing, such as flushing buffered events, return credits to a
        /// global pool, or process signaling related to load balancing.
        const DOES_NOT_REQUIRE_MAINTENCE =  0b10000000000;
    }
}

pub fn get_tx_capabilities_by_id(
    eventdev_id: EventDeviceId,
    ethdev_id: EthdevPortId,
) -> EventDevTxCapabilities {
    let mut caps: u32 = 0;
    let ret =
        unsafe { rte_event_eth_tx_adapter_caps_get(eventdev_id, ethdev_id, &mut caps as *mut u32) };
    assert!(ret == 0, "Invalid capabilities for rx port {ethdev_id}");

    EventDevTxCapabilities::from_bits_truncate(caps)
}

pub mod tx_adapter {
    use std::backtrace::Backtrace;

    use dpdk_sys::{
        rte_event_eth_tx_adapter_create, rte_event_eth_tx_adapter_queue_add,
        rte_event_eth_tx_adapter_start, rte_event_eth_tx_adapter_stop, rte_event_port_conf, EINVAL,
        ENOMEM, RTE_EVENT_PORT_CFG_HINT_CONSUMER,
    };

    use crate::device::{
        eth::dev::EthdevPortId,
        event::{
            dev::{get_eventdev_info, EventDevConfigDriverError},
            EventDeviceId,
        },
    };

    pub type TxAdapterId = u8;

    #[derive(Debug, thiserror::Error)]
    pub enum NewTxAdapterErrors {
        #[error("Invalid ethernet device id {ethdev_id}")]
        InvalidEtherDevice { ethdev_id: u8, backtrace: Backtrace },
        #[error("OOM: Ran out of memory allocating space for port configuration.")]
        OOM { backtrace: Backtrace },
        #[error("Unable to get eventdev info for event device")]
        EventDevInfoError(#[from] EventDevConfigDriverError),
    }

    #[derive(Debug, thiserror::Error)]
    pub enum AddQueueToTxAdapterErrors {
        #[error("While rx adapter {adapter_id} was trying to add a queue to event device {event_device_id} to handle input from port {ether_device_id}, restart was rejected, causing a configuration error.")]
        ConfigurationAndRestartError {
            adapter_id: TxAdapterId,
            event_device_id: EventDeviceId,
            ether_device_id: EthdevPortId,
            backtrace: Backtrace,
        },
    }

    pub struct TxAdapter {
        id: TxAdapterId,
    }

    impl TxAdapter {
        pub fn new(
            adapter_id: TxAdapterId,
            eventdev_id: EventDeviceId,
        ) -> Result<TxAdapter, NewTxAdapterErrors> {
            let dev_info = get_eventdev_info(eventdev_id)?;

            let mut event_port_config = rte_event_port_conf {
                new_event_threshold: dev_info.max_num_events,
                dequeue_depth: dev_info.max_event_port_dequeue_depth as u16,
                enqueue_depth: dev_info.max_event_port_enqueue_depth as u16,
                event_port_cfg: RTE_EVENT_PORT_CFG_HINT_CONSUMER,
            };

            let err = unsafe {
                rte_event_eth_tx_adapter_create(adapter_id, eventdev_id, &mut event_port_config)
            };

            match -err as u32 {
                ENOMEM => Err(NewTxAdapterErrors::OOM {
                    backtrace: Backtrace::capture(),
                }),
                0 => Ok(TxAdapter { id: adapter_id }),
                x => unimplemented!("Unknown error code {x}"),
            }
        }

        pub fn add_tx_queue(&mut self, eth_dev_id: EthdevPortId, ethdev_queue_id: i32) {
            let err =
                unsafe { rte_event_eth_tx_adapter_queue_add(self.id, eth_dev_id, ethdev_queue_id) };
            match -err as u32 {
                0 => {}
                EINVAL => {
                    panic!("Check logs");
                }
                ENOMEM => {
                    panic!("OOM");
                }
                x => {
                    unimplemented!("Unknown error code {x}");
                }
            }
        }

        pub fn start_sending_packets(self) {
            let err = unsafe { rte_event_eth_tx_adapter_start(self.id) };
            match -err as u32 {
                0 => {}
                EINVAL => unreachable!("Adapter ID should always be a real adapter at this point"),
                x => unimplemented!("Unknown error code {x}"),
            }
        }
    }

    pub fn stop_tx_adapter(id: TxAdapterId) {
        unsafe { rte_event_eth_tx_adapter_stop(id) };
    }
}
