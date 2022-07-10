use anyhow::Context;
use dpdk::{
    self,
    config::{DPDKConfig, VirtualDevice},
    device::{eth::{
        dev::{iter_ports, setup_port_queues, start_port, EthDriverError},
        rx::receive_burst,
    }, event::dev::{configure_eventdev_simple, event_queue_config::EventQueueConfig, event_port_config::{EventPortConfig, link_port_to_queue}, start_event_dev}},
    eal::{current_lcore_id, dpdk_exit},
    memory::pktmbuf_pool::PktMbufPool,
    raw::{
        rte_eal_cleanup, rte_eal_mp_wait_lcore, rte_eth_conf, rte_eth_rxmode, rte_mbuf, rte_pktmbuf_free_bulk, RTE_ETHER_MAX_LEN,
    },
};
use libc::{c_int, c_void};
use std::{mem::MaybeUninit, ptr::null_mut};

extern "C" fn lcore_hello(_unused: *mut c_void) -> c_int {
    let lcore = current_lcore_id();
    if lcore != 1 {
        println!("Lcore {} exiting", lcore);
        return 0;
    }

    let mut pkt_buffer: [&mut rte_mbuf; 32] = unsafe { MaybeUninit::zeroed().assume_init() };

    loop {
        for port in iter_ports() {
            let num_packets: usize = receive_burst::<32>(port, 0, &mut pkt_buffer) as usize;

            for i in 0..num_packets {
                let pkt_mbuf = &pkt_buffer[i];
                println!("Got packet on {}", pkt_mbuf.port);
            }
            unsafe {
                rte_pktmbuf_free_bulk(std::mem::transmute(&pkt_buffer), num_packets as u32);
            }
        }
    }
    return 0;
}

fn port_init(port: u16, pool: &mut PktMbufPool) -> Result<(), EthDriverError> {
    let port_conf = rte_eth_conf {
        rxmode: rte_eth_rxmode {
            mtu: RTE_ETHER_MAX_LEN,
            ..Default::default()
        },
        ..Default::default()
    };

    setup_port_queues(port, pool, 1, 1, &port_conf, 128, 512)?;

    start_port(port)?;

    Ok(())
}

fn main() -> Result<(), anyhow::Error> {
    DPDKConfig {
        cores: dpdk::config::CoreConfig::List(vec![1, 2, 3, 4]),
        main_lcore: None,
        service_core_mask: None,
        pci_options: dpdk::config::PCIOptions::NoPCI,
        virtual_devices: vec![
            VirtualDevice::with_driver("")
        ],
        num_memory_channels: Some(1),
    }
    .apply()
    .expect("Error configuring EAL");

    const NUM_EVENT_PORTS: u16 = 4;
    const NUM_EVENT_QUEUES: u16 = 2;

    configure_eventdev_simple(0, NUM_EVENT_PORTS as u8, NUM_EVENT_QUEUES as u8)
        .context("Failed to configure event device 0")?;
    
    let queue_config = EventQueueConfig::default();

    for i in 0..NUM_EVENT_QUEUES {
        queue_config.apply_to_eventdev_queue(0, i)?;
    }
    

    let port_config = EventPortConfig::default();

    for i in 0..NUM_EVENT_PORTS {
        port_config.setup_port(0, i)?;
    }

    link_port_to_queue(0, 0, &[0])?;
    link_port_to_queue(0, 0, &[0])?;

    start_event_dev()?;

    dpdk::raw::rte_lcore_foreach_worker(|lcore_id| unsafe {
        dpdk::raw::rte_eal_remote_launch(Some(lcore_hello), null_mut(), lcore_id);
    });

    // let nb_ports = dpdk::device::eth::dev::num_ports_available();

    // println!("There are {} ports available", nb_ports);

    // let mut mbuf_pool =
    //     dpdk::memory::pktmbuf_pool::PktMbufPool::new("MBUF_POOL\0", 8191 + nb_ports as u32, 250)
    //         .expect("Unable to intialize membuf pool.");

    // for port in iter_ports() {
    //     port_init(port, &mut mbuf_pool)?;
    // }

    // lcore_hello(null_mut());

    unsafe {
        rte_eal_mp_wait_lcore();

        rte_eal_cleanup();
    }
    dpdk_exit(0, "Success");
}
