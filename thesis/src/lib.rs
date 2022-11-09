#![feature(once_cell)]
#![allow(incomplete_features)]
#![feature(generic_const_exprs)]
#![feature(allocator_api)]
#![feature(new_uninit)]
#![feature(test)]
#![feature(btreemap_alloc)]
#![feature(linked_list_cursors)]
// Only for BTree Interval Set
#![feature(btree_drain_filter)]
// Only for Bit Flag Interval Set
#![feature(int_roundings)]
#![feature(array_zip)]
#![feature(binary_heap_into_iter_sorted)]

use dpdk::device::event::EventDeviceId;

pub mod log;
pub mod message;
pub mod prefix;
pub mod semaphore;
pub mod state;
pub mod sync_provider;
#[cfg(test)]
pub mod testing;
pub mod workers;
pub mod db;

#[cfg(test)]
extern crate test;

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

use dpdk::{
    self,
    config::{DPDKConfig, IOVAMode, PCIAddress, VirtualDevice},
    device::{
        eth::dev::{
            get_ethdev_port_info, setup_port_queues, start_port, EthdevPortId, EventQueueId,
        },
        event::EventPortId,
    },
    raw::{
        rte_eal_cleanup, rte_eal_mp_wait_lcore, rte_eth_conf, rte_eth_rxmode, rte_trace_save,
        RTE_ETH_LINK_SPEED_AUTONEG,
    },
};

use semaphore::SpinSemaphore;
use std::{collections::HashMap, mem::MaybeUninit, ptr::null_mut, sync::atomic::AtomicBool};
use workers::lcore_init;

/// Services are set up
static SERVICE_SETUP_DONE: AtomicBool = AtomicBool::new(false);

/// Time to terminate
static TERMINATE: AtomicBool = AtomicBool::new(false);
static RUNNING: SpinSemaphore = SpinSemaphore::new(3);

const MAX_CLIENTS: usize = 5;

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
            allowed_devices: vec![PCIAddress {
                domain: 0,
                bus: 2,
                device: 0,
                function: 0,
            }],
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
        iova_mode: Some(IOVAMode::PA),
        // iova_mode: Some(IOVAMode::VA),
    }
    .apply()
    .expect("Error configuring EAL");

    let port_info = get_ethdev_port_info(0).expect("Unable to get port info for port 0");

    let ena_offloads = 1 << 3 | // RTE_ETH_RSS_NONFRAG_IPV4_TCP
    1 << 4 | // RTE_ETH_RSS_NONFRAG_IPV4_UDP
    1 << 10 | // RTE_ETH_RSS_NONFRAG_IPV4_UDP
    1 << 11; // RTE_ETH_RSS_NONFRAG_IPV4_UDP

    #[cfg(feature = "ena")]
    let config = rte_eth_conf {
        link_speeds: RTE_ETH_LINK_SPEED_AUTONEG,
        rxmode: rte_eth_rxmode {
            mq_mode: dpdk::raw::rte_eth_rx_mq_mode::RTE_ETH_MQ_RX_RSS,
            mtu: 9001,
            max_lro_pkt_size: 9031,
            split_hdr_size: 0,
            offloads: port_info.rx_offload_capa,
            ..Default::default()
        },
        txmode: rte_eth_txmode {
            mq_mode: dpdk::raw::rte_eth_tx_mq_mode::RTE_ETH_MQ_TX_NONE,
            offloads: port_info.tx_offload_capa,
            ..Default::default()
        },
        rx_adv_conf: rte_eth_conf__bindgen_ty_1 {
            rss_conf: rte_eth_rss_conf {
                rss_key: std::ptr::null_mut(),
                rss_key_len: 40,
                rss_hf: ena_offloads & port_info.flow_type_rss_offloads,
            },
            ..unsafe { MaybeUninit::zeroed().assume_init() }
        },
        ..unsafe { MaybeUninit::zeroed().assume_init() }
    };

    #[cfg(feature = "virtio")]
    let config = rte_eth_conf {
        link_speeds: RTE_ETH_LINK_SPEED_AUTONEG,
        rxmode: rte_eth_rxmode {
            mq_mode: dpdk::raw::rte_eth_rx_mq_mode::RTE_ETH_MQ_RX_NONE,
            mtu: 9001,
            max_lro_pkt_size: 9031,
            split_hdr_size: 0,
            offloads: port_info.rx_offload_capa,
            ..Default::default()
        },
        ..unsafe { MaybeUninit::zeroed().assume_init() }
    };

    match setup_port_queues(0, 1, 1, &config, 32768, 32768) {
        Ok(()) => {}
        Err(e) => panic!("{e:#?}"),
    };
    start_port(0).expect("Unable to start port 0");
}

pub fn entrypoint() -> Result<(), anyhow::Error> {
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
