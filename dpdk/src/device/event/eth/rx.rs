use crate::device::{event::EventDeviceId, eth::dev::PortId};
use bitflags::bitflags;
use dpdk_sys::{rte_event_eth_rx_adapter_caps_get, rte_event_eth_tx_adapter_caps_get};

bitflags!{
    #[repr(C)]
    pub struct EventDevTxCapabilities: u32 {
        /// Event scheduling prioritization is based on the priority associated with each event queue.
        const queue_qos_scheduling = 0b0001;
        /// Event scheduling prioritization is based on the priority associated with
        /// each event. Priority of each event is supplied in *rte_event* structure
        /// on each enqueue operation.
        const event_qos_scheduling = 0b0010;
        /// Event device operates in distributed scheduling mode.
        /// In distributed scheduling mode, event scheduling happens in HW or
        /// rte_event_dequeue_burst() or the combination of these two.
        /// If the flag is not set then eventdev is centralized and thus needs a
        /// dedicated service core that acts as a scheduling thread .
        const distributed_scheduling = 0b0100;
        /// Event device is capable of enqueuing events of any type to any queue.
        /// If this capability is not set, the queue only supports events of the
        ///  `RTE_SCHED_TYPE_` type that it was created with.
        const queue_all_event_types = 0b1000;
        /// Event device is capable of operating in burst mode for enqueue(forward,
        /// release) and dequeue operation. If this capability is not set, application
        /// still uses the rte_event_dequeue_burst() and rte_event_enqueue_burst() but
        /// PMD accepts only one event at a time.
        const event_dev_burst_mode = 0b10000;
        /// Event device ports support disabling the implicit release feature, in
        /// which the port will release all unreleased events in its dequeue operation.
        /// If this capability is set and the port is configured with implicit release
        /// disabled, the application is responsible for explicitly releasing events
        /// using either the RTE_EVENT_OP_FORWARD or the RTE_EVENT_OP_RELEASE event
        /// enqueue operations.
        const disable_implicit_release =  0b100000;
        // Event device is capable of operating in none sequential mode. The path
        // of the event is not necessary to be sequential. Application can change
        // the path of event at runtime. If the flag is not set, then event each event
        // will follow a path from queue 0 to queue 1 to queue 2 etc. If the flag is
        // set, events may be sent to queues in any order. If the flag is not set, the
        // eventdev will return an error when the application enqueues an event for a
        // qid which is not the next in the sequence.
        const nonsequential_mode =  0b1000000;
        // Event device is capable of configuring the queue/port link at runtime.
        // If the flag is not set, the eventdev queue/port link is only can be
        // configured during  initialization.
        const runtime_port_link_config =  0b10000000;
        /// Event device is capable of setting up the link between multiple queue
        /// with single port. If the flag is not set, the eventdev can only map a
        /// single queue to each port or map a single queue to many port.
        const multi_queue_to_port =  0b100000000;
        /// Event device preserves the flow ID from the enqueued
        /// event to the dequeued event if the flag is set. Otherwise,
        /// the content of this field is implementation dependent.
        const carry_flow_id =  0b1000000000;
        /// Event device *does not* require calls to rte_event_maintain().
        /// An event device that does not set this flag requires calls to
        /// rte_event_maintain() during periods when neither
        /// rte_event_dequeue_burst() nor rte_event_enqueue_burst() are called
        /// on a port. This will allow the event device to perform internal
        /// processing, such as flushing buffered events, return credits to a
        /// global pool, or process signaling related to load balancing.
        const does_not_require_maintence =  0b10000000000;
    }

    #[repr(C)]
    pub struct EventDevRxCapabilities: u32 {
        /// This flag is sent when the packet transfer mechanism is in HW.
        /// Ethdev can send packets to the event device using internal event port.
        const hw_packet_transfer = 0b0001;
        /// Adapter supports multiple event queues per ethdev. Every ethdev
        /// Rx queue can be connected to a unique event queue.
        const multiple_event_queues_per_ethdev = 0b0010;
        /// The application can override the adapter generated flow ID in the
        /// event. This flow ID can be specified when adding an ethdev Rx queue
        /// to the adapter using the ev.flow_id member.
        /// @see ```rte_event_eth_rx_adapter_queue_conf::ev```
        /// @see ```rte_event_eth_rx_adapter_queue_conf::rx_queue_flags```
        const allow_override_generated_flow_id = 0b0100;
        /// Adapter supports event vectorization per ethdev.
        const per_ethdev_event_vectorization = 0b1000;
    }
}

pub fn get_rx_capabilities_by_id(eventdev_id: EventDeviceId, ethdev_id: PortId) -> EventDevRxCapabilities {
    let mut caps: u32 = 0;
    let ret = unsafe { 
        rte_event_eth_rx_adapter_caps_get(eventdev_id, ethdev_id, &mut caps as *mut u32)
     };
     assert!(ret == 0, "Invalid capabilities for rx port {ethdev_id}");

    return EventDevRxCapabilities::from_bits_truncate(caps);
}

pub fn get_tx_capabilities_by_id(eventdev_id: EventDeviceId, ethdev_id: PortId) -> EventDevTxCapabilities {
    let mut caps: u32 = 0;
    let ret = unsafe { 
        rte_event_eth_tx_adapter_caps_get(eventdev_id, ethdev_id, &mut caps as *mut u32)
     };
     assert!(ret == 0, "Invalid capabilities for rx port {ethdev_id}");

    return EventDevTxCapabilities::from_bits_truncate(caps);
}