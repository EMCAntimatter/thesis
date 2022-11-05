use crate::device::{eth::dev::EthdevPortId, event::EventDeviceId};
use bitflags::bitflags;
use dpdk_sys::rte_event_eth_rx_adapter_caps_get;

bitflags! {
    #[repr(C)]
    pub struct EventDevRxCapabilities: u32 {
        /// This flag is sent when the packet transfer mechanism is in HW.
        /// Ethdev can send packets to the event device using internal event port.
        const HW_PACKET_TRANSFER = 0b0001;
        /// Adapter supports multiple event queues per ethdev. Every ethdev
        /// Rx queue can be connected to a unique event queue.
        const MULTIPLE_EVENT_QUEUES_PER_ETHDEV = 0b0010;
        /// The application can override the adapter generated flow ID in the
        /// event. This flow ID can be specified when adding an ethdev Rx queue
        /// to the adapter using the ev.flow_id member.
        /// @see ```rte_event_eth_rx_adapter_queue_conf::ev```
        /// @see ```rte_event_eth_rx_adapter_queue_conf::rx_queue_flags```
        const ALLOW_OVERRIDE_GENERATED_FLOW_ID = 0b0100;
        /// Adapter supports event vectorization per ethdev.
        const PER_ETHDEV_EVENT_VECTORIZATION = 0b1000;
    }
}

pub fn get_rx_capabilities_by_id(
    eventdev_id: EventDeviceId,
    ethdev_id: EthdevPortId,
) -> EventDevRxCapabilities {
    let mut caps: u32 = 0;
    let ret =
        unsafe { rte_event_eth_rx_adapter_caps_get(eventdev_id, ethdev_id, &mut caps as *mut u32) };
    assert!(ret == 0, "Invalid capabilities for rx port {ethdev_id}");

    EventDevRxCapabilities::from_bits_truncate(caps)
}

pub mod rx_adapter {
    use std::backtrace::Backtrace;

    use dpdk_sys::{
        rte_event_eth_rx_adapter_create, rte_event_eth_rx_adapter_queue_add,
        rte_event_eth_rx_adapter_queue_conf, rte_event_eth_rx_adapter_start,
        rte_event_eth_rx_adapter_stop, rte_event_port_conf, rte_mempool, EINVAL, EIO, ENOMEM,
        RTE_EVENT_PORT_CFG_HINT_PRODUCER,
    };

    use crate::device::{
        eth::dev::EthdevPortId,
        event::{
            dev::{get_eventdev_info, EventDevConfigDriverError},
            event::Event,
            EventDeviceId,
        },
    };

    pub type RxAdapterId = u8;

    #[derive(Debug, thiserror::Error)]
    pub enum NewRxAdapterErrors {
        #[error("Invalid ethernet device id {ethdev_id}")]
        InvalidEtherDevice { ethdev_id: u8, backtrace: Backtrace },
        #[error("OOM: Ran out of memory allocating space for port configuration.")]
        OOM { backtrace: Backtrace },
        #[error("Unable to get eventdev info for event device")]
        EventDevInfoError(#[from] EventDevConfigDriverError),
    }

    pub enum RxAdapterQueueConfigExtensions {
        None,
        Vector {
            vector_size: u16,
            vector_timeout_ns: u64,
            vector_mempool: *mut rte_mempool,
        },
    }

    pub struct RxAdapterQueueConfig {
        pub rx_queue_flags: u32,
        pub servicing_weight: u16,
        pub event: Event,
        pub event_buf_size: u16,
        pub extensions: RxAdapterQueueConfigExtensions,
    }

    impl From<RxAdapterQueueConfig> for rte_event_eth_rx_adapter_queue_conf {
        fn from(val: RxAdapterQueueConfig) -> Self {
            let conf = rte_event_eth_rx_adapter_queue_conf {
                rx_queue_flags: val.rx_queue_flags,
                servicing_weight: val.servicing_weight,
                ev: (&val.event).into(),
                vector_sz: 0,
                vector_timeout_ns: 0,
                vector_mp: std::ptr::null_mut(),
                event_buf_size: val.event_buf_size,
            };

            match val.extensions {
                RxAdapterQueueConfigExtensions::None => conf,
                RxAdapterQueueConfigExtensions::Vector {
                    vector_size,
                    vector_timeout_ns,
                    vector_mempool,
                } => rte_event_eth_rx_adapter_queue_conf {
                    vector_sz: vector_size,
                    vector_timeout_ns,
                    vector_mp: vector_mempool,
                    ..conf
                },
            }
        }
    }

    #[derive(Debug, thiserror::Error)]
    pub enum AddQueueToRxAdapterErrors {
        #[error("While rx adapter {adapter_id} was trying to add a queue to event device {event_device_id} to handle input from port {ether_device_id}, restart was rejected, causing a configuration error.")]
        ConfigurationAndRestartError {
            adapter_id: RxAdapterId,
            event_device_id: EventDeviceId,
            ether_device_id: EthdevPortId,
            backtrace: Backtrace,
        },
    }

    pub struct RxAdapter {
        adapter_id: RxAdapterId,
        event_device_id: EventDeviceId,
        ether_device_id: EthdevPortId,
    }

    impl RxAdapter {
        pub fn new(
            adapter_id: RxAdapterId,
            eventdev_id: EventDeviceId,
            ethdev_id: EthdevPortId,
        ) -> Result<RxAdapter, NewRxAdapterErrors> {
            let dev_info = get_eventdev_info(eventdev_id)?;

            let mut event_port_config = rte_event_port_conf {
                new_event_threshold: dev_info.max_num_events,
                dequeue_depth: dev_info.max_event_port_dequeue_depth as u16,
                enqueue_depth: dev_info.max_event_port_enqueue_depth as u16,
                event_port_cfg: RTE_EVENT_PORT_CFG_HINT_PRODUCER,
            };

            let err = unsafe {
                rte_event_eth_rx_adapter_create(adapter_id, ethdev_id as u8, &mut event_port_config)
            };

            match -err as u32 {
                EINVAL => Err(NewRxAdapterErrors::InvalidEtherDevice {
                    ethdev_id: ethdev_id as u8,
                    backtrace: Backtrace::capture(),
                }),
                ENOMEM => Err(NewRxAdapterErrors::OOM {
                    backtrace: Backtrace::capture(),
                }),
                0 => Ok(RxAdapter {
                    adapter_id,
                    event_device_id: eventdev_id,
                    ether_device_id: ethdev_id,
                }),
                x => unimplemented!("Unknown error code {x}"),
            }
        }

        pub fn add_rx_queue(
            &mut self,
            queue_config: RxAdapterQueueConfig,
            rx_queue_id: i32,
        ) -> Result<(), AddQueueToRxAdapterErrors> {
            let config = &queue_config.into();
            let err = unsafe {
                rte_event_eth_rx_adapter_queue_add(
                    self.adapter_id,
                    self.ether_device_id,
                    rx_queue_id,
                    config,
                )
            };
            match -err as u32 {
                0 => Ok(()),
                EIO => Err(AddQueueToRxAdapterErrors::ConfigurationAndRestartError {
                    adapter_id: self.adapter_id,
                    event_device_id: self.event_device_id,
                    ether_device_id: self.ether_device_id,
                    backtrace: Backtrace::capture(),
                }),
                EINVAL => {
                    panic!("Check logs")
                }
                ENOMEM => {
                    panic!("OOM")
                }
                x => unimplemented!("Unknown error code {x}"),
            }
        }

        pub fn start_forwarding_packets(self) {
            let err = unsafe { rte_event_eth_rx_adapter_start(self.adapter_id) };
            match -err as u32 {
                0 => {}
                EINVAL => unreachable!("Adapter ID should always be a real adapter at this point"),
                x => unimplemented!("Unknown error code {x}"),
            }
        }
    }

    pub fn stop_rx_adapter(id: RxAdapterId) {
        unsafe { rte_event_eth_rx_adapter_stop(id) };
    }
}
