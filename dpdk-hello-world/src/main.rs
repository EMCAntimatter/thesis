#![feature(once_cell)]
#![feature(generic_const_exprs)]
#![feature(allocator_api)]
#![feature(new_uninit)]


pub mod semaphore;
pub mod workers;
pub mod message;
pub mod log;


use dpdk::{
    self,
    config::{DPDKConfig, VirtualDevice, PCIAddress, IOVAMode},
    device::{
        eth::dev::{EthdevPortId, EventQueueId, configure_port, setup_port_queues},
        event::{
            eth::{rx::rx_adapter::stop_rx_adapter, tx::tx_adapter::stop_tx_adapter},
            EventDeviceId, EventPortId,
        },
    },
    raw::{rte_eal_cleanup, rte_eal_mp_wait_lcore, rte_trace_save, rte_eth_conf, RTE_ETH_LINK_SPEED_AUTONEG, rte_eth_rxmode},
};

use semaphore::SpinSemaphore;
use std::{
    collections::HashMap,
    ptr::null_mut,
    sync::atomic::{AtomicBool},
};
use workers::lcore_init;

/// Services are set up
static SERVICE_SETUP_DONE: AtomicBool = AtomicBool::new(false);

/// Time to terminate
static TERMINATE: AtomicBool = AtomicBool::new(false);
static RUNNING: SpinSemaphore = SpinSemaphore::new(3);

#[macro_export]
macro_rules! swap_with_uninit {
    ($e:expr) => {
        std::mem::replace($e, unsafe { MaybeUninit::uninit().assume_init() })
    };
}

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
            // VirtualDevice {
            //     driver: "net_pcap".to_string(),
            //     id: 0,
            //     options: HashMap::from([
            //         (
            //             "rx_pcap".to_string(),
            //             "./inputs/a.pcap".to_string(),
            //         ),
            //         (
            //             "tx_pcap".to_string(),
            //             "./outputs/a.pcap".to_string(),
            //         ),
            //         ("infinite_rx".to_string(), "1".to_string()),
            //     ]),
            // },
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

    let config = rte_eth_conf {
        link_speeds: todo!(),
        rxmode: todo!(),
        txmode: todo!(),
        lpbk_mode: todo!(),
        rx_adv_conf: todo!(),
        tx_adv_conf: todo!(),
        dcb_capability_en: todo!(),
        fdir_conf: todo!(),
        intr_conf: todo!(),
    };

    let port_conf = rte_eth_conf {
        link_speeds: todo!(),
        rxmode: todo!(),
        txmode: todo!(),
        lpbk_mode: todo!(),
        rx_adv_conf: todo!(),
        tx_adv_conf: todo!(),
        dcb_capability_en: todo!(),
        fdir_conf: todo!(),
        intr_conf: todo!(),
    };

    let config = rte_eth_conf {
        link_speeds: RTE_ETH_LINK_SPEED_AUTONEG,
        rxmode: rte_eth_rxmode {
            mq_mode: dpdk::raw::rte_eth_rx_mq_mode::RTE_ETH_MQ_RX_RSS,
            mtu: todo!(),
            max_lro_pkt_size: todo!(),
            split_hdr_size: todo!(),
            offloads: todo!(),
            reserved_64s: todo!(),
            reserved_ptrs: todo!(),
        },
        txmode: todo!(),
        lpbk_mode: todo!(),
        rx_adv_conf: todo!(),
        tx_adv_conf: todo!(),
        dcb_capability_en: todo!(),
        fdir_conf: todo!(),
        intr_conf: todo!(),
    };

    setup_port_queues(0, 1, 1, )
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
const ETHDEV_QUEUE_ID: u16 = 0;

fn main() -> Result<(), anyhow::Error> {
    apply_config();

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

    unsafe {
        rte_eal_mp_wait_lcore();
        rte_eal_cleanup();
    }
    Ok(())
    // dpdk_exit(0, "Success");
}
