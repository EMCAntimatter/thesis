use std::{backtrace::Backtrace, mem::MaybeUninit};

use dpdk_sys::{
    rte_event_dev_config, rte_event_dev_configure, rte_event_dev_info, rte_event_dev_info_get,
    rte_event_port_link, rte_event_queue_conf, RTE_EVENT_DEV_PRIORITY_NORMAL,
    RTE_SCHED_TYPE_ATOMIC, rte_event_dev_start, ESTALE, ENOLINK,
};
use libc::EDQUOT;

use crate::{
    device::eth::dev::{iter_ports, PortId, QueueId},
    eal::RteErrnoValue,
};

use super::{
    eth::rx::{get_rx_capabilities_by_id, EventDevRxCapabilities},
    EventDeviceId,
};

use derive_builder::Builder;

pub fn do_capability_setup() {
    let mut capabilities = EventDevRxCapabilities::empty();

    for ethdev_id in iter_ports() {
        capabilities = capabilities & (get_rx_capabilities_by_id(0, ethdev_id))
    }
    let pipeline_rx_capabilities = capabilities | EventDevRxCapabilities::hw_packet_transfer;

    let mut eventdev_info: rte_event_dev_info = unsafe { MaybeUninit::zeroed().assume_init() };

    unsafe {
        rte_event_dev_info_get(0, &mut eventdev_info);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EventDevConfigDriverError {
    #[error("Failed to configure device {eventdev_id}, recieved driver error {driver_error}")]
    DriverConfigurationFailureError {
        eventdev_id: EventDeviceId,
        driver_error: i32,
        backtrace: Backtrace,
    },
    #[error("Invalid configuration")]
    InvalidConfiguration(#[from] EventDevConfigBuilderError, Backtrace),
}

#[derive(Debug, Default, Builder)]
#[builder(pattern = "owned", default)]
pub struct EventDevConfig {
    dequeue_timeout: u32,
    event_limit_amount: i32,
    num_event_queues: u8,
    num_ports: u8,
    num_event_queue_flows: u32,
    port_dequeue_depth: u32,
    port_enqueue_depth: u32,
    event_dev_cfg: u32,
    num_single_link_port_event_queues: u8,
}

pub fn configure_eventdev_simple(
    device_id: EventDeviceId,
    num_ports: u8,
    num_event_queues: u8,
) -> Result<(), EventDevConfigDriverError> {
    let config = EventDevConfigBuilder::default()
        .num_ports(num_ports)
        .num_event_queues(num_event_queues)
        .build()?;
    configure_eventdev(device_id, &config)
}

pub fn configure_eventdev(
    device_id: EventDeviceId,
    config: &EventDevConfig,
) -> Result<(), EventDevConfigDriverError> {
    let config = rte_event_dev_config {
        dequeue_timeout_ns: config.dequeue_timeout,
        nb_events_limit: config.event_limit_amount,
        nb_event_queues: config.num_event_queues,
        nb_event_ports: config.num_ports,
        nb_event_queue_flows: config.num_event_queue_flows,
        nb_event_port_dequeue_depth: config.port_dequeue_depth,
        nb_event_port_enqueue_depth: config.port_enqueue_depth,
        event_dev_cfg: config.event_dev_cfg,
        nb_single_link_event_port_queues: config.num_single_link_port_event_queues,
    };
    let ret = unsafe { rte_event_dev_configure(device_id, &config) };

    if ret == 0 {
        Ok(())
    } else {
        Err(EventDevConfigDriverError::DriverConfigurationFailureError {
            eventdev_id: device_id,
            driver_error: ret,
            backtrace: Backtrace::capture(),
        })
    }
}

pub mod event_queue_config {
    use std::backtrace::Backtrace;

    use dpdk_sys::{
        rte_event_queue_conf, rte_event_queue_setup, RTE_EVENT_QUEUE_CFG_SINGLE_LINK,
        RTE_SCHED_TYPE_ATOMIC, RTE_SCHED_TYPE_ORDERED, RTE_SCHED_TYPE_PARALLEL,
    };

    use crate::device::{eth::dev::QueueId, event::EventDeviceId};

    #[derive(Debug, thiserror::Error)]
    pub enum EventDevQueueConfigError {
        #[error("Invalid config for device {device_id}, queue {queue_id}, rte_event_queue_setup returned {code}")]
        ConfigurationFailure {
            device_id: EventDeviceId,
            queue_id: QueueId,
            code: i32,
            backtrace: Backtrace,
        },
    }

    #[derive(Debug, Clone, Copy)]
    pub enum EventQueueConfigType {
        ORDERED,
        ATOMIC {
            num_flows: u32,
            num_order_sequences: u32,
        },
        PARALLEL,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct EventQueueConfig {
        pub config_type: EventQueueConfigType,
        pub is_single_link: bool,
        pub priority: u8,
    }

    impl Default for EventQueueConfig {
        fn default() -> Self {
            Self {
                config_type: EventQueueConfigType::ORDERED,
                is_single_link: true,
                priority: 0,
            }
        }
    }

    impl EventQueueConfig {
        pub fn apply_to_eventdev_queue(
            &self,
            device_id: EventDeviceId,
            queue_id: QueueId,
        ) -> Result<(), EventDevQueueConfigError> {
            let queue_conf: rte_event_queue_conf = self.into();
            let ret = unsafe { rte_event_queue_setup(device_id, queue_id as u8, &queue_conf) };
            if ret == 0 {
                Ok(())
            } else {
                Err(EventDevQueueConfigError::ConfigurationFailure {
                    device_id,
                    queue_id,
                    code: ret,
                    backtrace: Backtrace::capture(),
                })
            }
        }
    }

    impl Into<rte_event_queue_conf> for &EventQueueConfig {
        fn into(self) -> rte_event_queue_conf {
            let event_queue_cfg = if self.is_single_link {
                RTE_EVENT_QUEUE_CFG_SINGLE_LINK
            } else {
                0
            };
            match self.config_type {
                EventQueueConfigType::ORDERED => rte_event_queue_conf {
                    nb_atomic_flows: 0,
                    nb_atomic_order_sequences: 0,
                    event_queue_cfg,
                    schedule_type: RTE_SCHED_TYPE_ORDERED as u8,
                    priority: self.priority,
                },
                EventQueueConfigType::ATOMIC {
                    num_flows,
                    num_order_sequences,
                } => rte_event_queue_conf {
                    nb_atomic_flows: num_flows,
                    nb_atomic_order_sequences: num_order_sequences,
                    event_queue_cfg,
                    schedule_type: RTE_SCHED_TYPE_ATOMIC as u8,
                    priority: self.priority,
                },
                EventQueueConfigType::PARALLEL => rte_event_queue_conf {
                    nb_atomic_flows: 0,
                    nb_atomic_order_sequences: 0,
                    event_queue_cfg,
                    schedule_type: RTE_SCHED_TYPE_PARALLEL as u8,
                    priority: self.priority,
                },
            }
        }
    }
}
pub mod event_port_config {
    use std::backtrace::Backtrace;

    use dpdk_sys::{
        rte_event_port_conf, rte_event_port_link, rte_event_port_setup,
        RTE_EVENT_DEV_PRIORITY_NORMAL,
    };
    use libc::EDQUOT;

    use crate::{
        device::{
            eth::dev::{PortId, QueueId},
            event::EventDeviceId,
        },
        eal::RteErrnoValue,
    };

    #[derive(Debug, thiserror::Error)]
    pub enum EventPortConfigError {
        #[error("Application tried to link port {port_id} on device {device_id} configured with RTE_EVENT_QUEUE_CFG_SINGLE_LINK to more than one event ports")]
        TooManyLinksToQueue {
            device_id: EventDeviceId,
            port_id: PortId,
            backtrace: Backtrace,
        },
        #[error(
            "Error configuring port {port_id} on device {device_id}. Got error code {error_code}."
        )]
        GenericPortConfigError {
            device_id: EventDeviceId,
            port_id: PortId,
            error_code: i32,
            backtrace: Backtrace,
        },
        #[error(
            "Invalid parameter for linking port {port_id} on device {device_id}, actually linked {num_linked}"
        )]
        InvalidLinkParameters {
            device_id: EventDeviceId,
            port_id: PortId,
            num_linked: u32,
            backtrace: Backtrace,
        },
    }

    pub struct EventPortConfig {
        new_event_threshold: i32,
        dequeue_depth: u16,
        enqueue_depth: u16,
        event_port_cfg: u32,
    }

    impl Default for EventPortConfig {
        fn default() -> Self {
            Self {
                new_event_threshold: 4096,
                dequeue_depth: 128,
                enqueue_depth: 128,
                event_port_cfg: 0,
            }
        }
    }

    impl EventPortConfig {
        pub fn setup_port(
            &self,
            device_id: EventDeviceId,
            port_id: PortId,
        ) -> Result<(), EventPortConfigError> {
            let config: rte_event_port_conf = self.into();
            let ret = unsafe { rte_event_port_setup(device_id as u8, port_id as u8, &config) };
            if ret == -1 * EDQUOT {
                Err(EventPortConfigError::TooManyLinksToQueue {
                    device_id,
                    port_id,
                    backtrace: Backtrace::capture(),
                })
            } else if ret < 0 {
                Err(EventPortConfigError::GenericPortConfigError {
                    device_id,
                    port_id,
                    error_code: ret,
                    backtrace: Backtrace::capture(),
                })
            } else {
                Ok(())
            }
        }
    }

    impl Into<rte_event_port_conf> for &EventPortConfig {
        fn into(self) -> rte_event_port_conf {
            rte_event_port_conf {
                new_event_threshold: self.new_event_threshold,
                dequeue_depth: self.dequeue_depth,
                enqueue_depth: self.enqueue_depth,
                event_port_cfg: self.event_port_cfg,
            }
        }
    }

    pub fn link_port_to_queue(
        device_id: EventDeviceId,
        port_id: PortId,
        queue_ids: &[u8],
    ) -> Result<(), EventPortConfigError> {
        RteErrnoValue::clear();
        let ret = unsafe {
            rte_event_port_link(
                device_id,
                port_id as u8,
                &queue_ids[0] as *const u8,
                &(RTE_EVENT_DEV_PRIORITY_NORMAL as u8),
                queue_ids.len() as u16,
            )
        };
        if ret != queue_ids.len() as i32 {
            match RteErrnoValue::most_recent() {
                RteErrnoValue::EDQUOT => Err(EventPortConfigError::TooManyLinksToQueue {
                    device_id,
                    port_id,
                    backtrace: Backtrace::capture(),
                }),
                RteErrnoValue::EINVAL => Err(EventPortConfigError::InvalidLinkParameters {
                    device_id,
                    port_id,
                    num_linked: ret as u32,
                    backtrace: Backtrace::capture(),
                }),
                other => {
                    panic!("Unhandled rte errono value {other:?}")
                }
            }
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EventDevStartErrors {
    #[error("Not all ports were started")]
    NotAllPortsStartedError(Backtrace),
    #[error("Not all queues are linked")]
    NotAllQueuesLinkedError(Backtrace)
}

pub fn start_event_dev() -> Result<(), EventDevStartErrors> {
    let ret = unsafe {
        rte_event_dev_start(0)
    };
    if ret == 0 {
        Ok(())
    } else if ret == -1 * ESTALE as i32 {
        Err(EventDevStartErrors::NotAllPortsStartedError(Backtrace::capture()))
    } else if ret == -1 * ENOLINK as i32{
        Err(EventDevStartErrors::NotAllQueuesLinkedError(Backtrace::capture()))
    } else {
        panic!("Unhandled error, {ret}");
    }
}