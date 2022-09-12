use std::backtrace::Backtrace;

use dpdk_sys::{
    rte_eth_conf, rte_eth_dev_configure, rte_eth_dev_count_avail, rte_eth_dev_socket_id,
    rte_eth_rx_queue_setup, rte_eth_tx_queue_setup, rte_socket_id, rte_eth_dev_start,
};


pub type EthdevPortId = u16;
pub type EventQueueId = u16;

#[derive(Debug, thiserror::Error)]
pub enum EthDriverError {
    #[error("Error starting port {port}, rx queue {queue}, received driver error {driver_error}")]
    PortRxQueueStartError {
        port: EthdevPortId,
        queue: EventQueueId,
        driver_error: i32,
        backtrace: Backtrace,
    },
    #[error("Error starting port {port}, tx queue {queue}, received driver error {driver_error}")]
    PortTxQueueStartError {
        port: EthdevPortId,
        queue: EventQueueId,
        driver_error: i32,
        backtrace: Backtrace,
    },
    #[error("Error configuring port {port}, received driver error {driver_error}")]
    PortConfigureError {
        port: EthdevPortId,
        driver_error: i32,
        backtrace: Backtrace,
    },
    #[error("Error starting port {port}, recieved driver error {driver_error}")]
    PortStartError {
        port: EthdevPortId,
        driver_error: i32,
        backtrace: Backtrace,
    }
}

use crate::memory::pktmbuf_pool::PktMbufPool;

pub fn num_ports_available() -> u16 {
    unsafe { rte_eth_dev_count_avail() }
}

pub fn iter_ports() -> std::ops::Range<u16> {
    0..num_ports_available()
}

pub fn socket_id() -> u32 {
    unsafe { rte_socket_id() }
}

pub fn socket_id_for_port(port_id: EthdevPortId) -> Option<u32> {
    let socket = unsafe { rte_eth_dev_socket_id(port_id) };
    if socket == -1 {
        None
    } else {
        Some(socket as u32)
    }
}

pub fn configure_port(
    port: EthdevPortId,
    num_rx_queues: u16,
    num_tx_queues: u16,
    port_conf: &rte_eth_conf,
) -> Result<(), EthDriverError> {
    let ret = unsafe { rte_eth_dev_configure(port, num_rx_queues, num_tx_queues, port_conf) };
    if ret < 0 {
        Err(EthDriverError::PortConfigureError {
            port,
            driver_error: ret,
            backtrace: Backtrace::capture()
        })
    } else {
        Ok(())
    }
}

pub fn setup_port_queues(
    port: EthdevPortId,
    pool: &mut PktMbufPool,
    num_rx_queues: u16,
    num_tx_queues: u16,
    port_conf: &rte_eth_conf,
    rx_ring_size: u16,
    tx_ring_size: u16,
) -> Result<(), EthDriverError> {
    configure_port(port, num_rx_queues, num_tx_queues, port_conf)?;

    let port_socket = socket_id_for_port(port).expect("Port had no socket id");

    for queue_id in 0..num_rx_queues {
        let driver_error = unsafe {
            rte_eth_rx_queue_setup(
                port,
                queue_id,
                rx_ring_size,
                port_socket,
                std::ptr::null(),
                pool.as_mut(),
            )
        };
        if driver_error < 0 {
            return Err(EthDriverError::PortRxQueueStartError {
                port,
                queue: queue_id,
                driver_error,
                backtrace: Backtrace::capture()
            });
        }
    }

    for queue_id in 0..num_tx_queues {
        let driver_error = unsafe {
            rte_eth_tx_queue_setup(
                port,
                queue_id,
                tx_ring_size,
                port_socket,
                std::ptr::null(),
            )
        };
        if driver_error < 0 {
            return Err(EthDriverError::PortTxQueueStartError {
                port,
                queue: queue_id,
                driver_error,
                backtrace: Backtrace::capture()
            });
        }
    }

    Ok(())
}

pub fn start_port(port: EthdevPortId) -> Result<(), EthDriverError> {
    let ret = unsafe { rte_eth_dev_start(port) };
    if ret < 0 {
        Err(EthDriverError::PortStartError { port, driver_error: ret, backtrace: Backtrace::capture() })
    } else {
        Ok(())
    }
}