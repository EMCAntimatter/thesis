#![feature(once_cell)]
#![feature(generic_const_exprs)]


pub mod event_handlers;
pub mod semaphore;
pub mod workers;
pub mod message;
pub mod log;

use anyhow::Context;
use dpdk::{
    self,
    config::{DPDKConfig, VirtualDevice, PCIAddress, IOVAMode},
    device::{
        eth::dev::{EthdevPortId, EventQueueId},
        event::{
            dev::{
                event_port_config::{link_port_to_queue, EventPortConfig},
                event_queue_config::EventQueueConfig,
                setup_eventdev_simple, start_event_dev,
            },
            eth::{rx::rx_adapter::stop_rx_adapter, tx::tx_adapter::stop_tx_adapter},
            EventDeviceId, EventPortId,
        },
    },
    raw::{rte_eal_cleanup, rte_eal_mp_wait_lcore, rte_trace_save, RTE_EVENT_DEV_PRIORITY_NORMAL},
};

use semaphore::SpinSemaphore;
use std::{
    collections::HashMap,
    ptr::null_mut,
    sync::atomic::{fence, AtomicBool},
};
use workers::{lcore_init, rx_adapter_worker, tx_adapter_worker};

/// Services are set up
static SERVICE_SETUP_DONE: AtomicBool = AtomicBool::new(false);

/// Time to terminate
static TERMINATE: AtomicBool = AtomicBool::new(false);
static RUNNING: SpinSemaphore = SpinSemaphore::new(3);

fn apply_config() {
    DPDKConfig {
        cores: dpdk::config::CoreConfig::List(vec![1, 2, 3]),
        main_lcore: None,
        service_core_mask: Some(0b11),
        pci_options: dpdk::config::PCIOptions::PCI {
            blocked_devices: vec![],
            allowed_devices: vec![
                PCIAddress { domain: 0, bus: 2, device: 0, function: 0 }
            ],
        },
        virtual_devices: vec![
            VirtualDevice {
                driver: "event_sw".to_string(),
                id: 0,
                options: HashMap::from([
                    // ("credit_quanta".to_string(), "64".to_string())
                ]),
            },
            VirtualDevice {
                driver: "net_pcap".to_string(),
                id: 0,
                options: HashMap::from([
                    (
                        "rx_pcap".to_string(),
                        "./inputs/a.pcap".to_string(),
                    ),
                    (
                        "tx_pcap".to_string(),
                        "./outputs/a.pcap".to_string(),
                    ),
                    ("infinite_rx".to_string(), "1".to_string()),
                ]),
            },
        ],
        num_memory_channels: None,
        enable_telemetry: true,
        // trace: Some(".*".to_string()),
        trace: None,
        // iova_mode: Some(IOVAMode::PA),
        iova_mode: Some(IOVAMode::PA),
    }
    .apply()
    .expect("Error configuring EAL");
}

// eventdev setup
const NUM_EVENT_PORTS: u16 = 3;
const NUM_EVENT_QUEUES: u16 = 2;

// ports
const RX_ADAPTER_INPUT_PORT_ID: EventPortId = 0;
const EVENT_HANDLER_PORT_ID: EventPortId = 1;
const TX_ADAPTER_INPUT_PORT_ID: EventPortId = 2;

// queues
const RX_ADAPTER_OUTPUT_QUEUE_ID: EventQueueId = 0;
const TX_ADAPTER_INPUT_QUEUE_ID: EventQueueId = 1;

// devs
const EVENTDEV_DEVICE_ID: EventDeviceId = 0;
const ETHDEV_PORT_ID: EthdevPortId = 0;

fn main() -> Result<(), anyhow::Error> {
    apply_config();

    setup_eventdev_simple(0, NUM_EVENT_PORTS as u8, NUM_EVENT_QUEUES as u8)
        .context("Failed to configure event device 0")?;

    let queue_config = EventQueueConfig {
        config_type: dpdk::device::event::dev::event_queue_config::EventQueueConfigType::PARALLEL,
        is_single_link: true,
        priority: RTE_EVENT_DEV_PRIORITY_NORMAL as u8,
    };

    for i in 0..NUM_EVENT_QUEUES {
        queue_config.apply_to_eventdev_queue(EVENTDEV_DEVICE_ID, i)?;
    }

    let mut input_port_config =
        EventPortConfig::default_for_port(EVENTDEV_DEVICE_ID, RX_ADAPTER_INPUT_PORT_ID);

    input_port_config.new_event_threshold = 1;

    input_port_config.setup_port(EVENTDEV_DEVICE_ID, RX_ADAPTER_INPUT_PORT_ID)?;

    let output_port_config =
        EventPortConfig::default_for_port(EVENTDEV_DEVICE_ID, EVENT_HANDLER_PORT_ID);

    output_port_config.setup_port(EVENTDEV_DEVICE_ID, EVENT_HANDLER_PORT_ID)?;
    output_port_config.setup_port(EVENTDEV_DEVICE_ID, TX_ADAPTER_INPUT_PORT_ID)?;

    link_port_to_queue(
        EVENTDEV_DEVICE_ID,
        EVENT_HANDLER_PORT_ID,
        &[RX_ADAPTER_OUTPUT_QUEUE_ID as u8],
    )?;
    link_port_to_queue(
        EVENTDEV_DEVICE_ID,
        TX_ADAPTER_INPUT_PORT_ID,
        &[TX_ADAPTER_INPUT_QUEUE_ID as u8],
    )?;

    start_event_dev(EVENTDEV_DEVICE_ID)?;

    rx_adapter_worker();
    tx_adapter_worker();

    fence(std::sync::atomic::Ordering::SeqCst);

    dpdk::raw::rte_lcore_foreach_worker(|lcore_id| unsafe {
        dpdk::raw::rte_eal_remote_launch(Some(lcore_init), null_mut(), lcore_id);
    });

    unsafe {
        rte_trace_save();
    }

    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).ok();

    TERMINATE.store(true, std::sync::atomic::Ordering::SeqCst);
    RUNNING.take_max(); // wait for everything to finish
    stop_tx_adapter(0);
    stop_rx_adapter(0);

    unsafe {
        rte_eal_mp_wait_lcore();
        rte_eal_cleanup();
    }
    Ok(())
    // dpdk_exit(0, "Success");
}
