use core::panic;
use std::{ptr::{slice_from_raw_parts, slice_from_raw_parts_mut}, sync::atomic::AtomicPtr, intrinsics::transmute};

use dpdk_sys::{rte_eth_rx_burst, rte_mbuf, RTE_MAX_ETHPORTS, RTE_MAX_QUEUES_PER_PORT, rte_eth_fp_ops, rte_eth_dev_is_valid_port, __rte_ethdev_trace_rx_burst, rte_eth_call_rx_callbacks};
use libc::c_void;

use super::dev::{PortId, QueueId};

pub fn receive_burst<const NUM_PACKETS: u16>(port_id: PortId, queue_id: QueueId, rx_buffer: &mut[&mut rte_mbuf; NUM_PACKETS as usize]) -> u16 {
    debug_assert!(u32::from(port_id) < RTE_MAX_ETHPORTS, "Invalid port id {port_id}, maximum is {RTE_MAX_ETHPORTS}.");
    debug_assert!(u32::from(queue_id) < RTE_MAX_QUEUES_PER_PORT, "Invalid queue id {queue_id}, maximum is {RTE_MAX_QUEUES_PER_PORT}");
    debug_assert!(unsafe { rte_eth_dev_is_valid_port(port_id) } == 1, "Invalid port id {port_id}");

    let queue_data_pointer: &rte_eth_fp_ops = unsafe { &rte_eth_fp_ops[port_id as usize] };
    let queue_data = unsafe {
        let queue_data_pointer_rxq_data = queue_data_pointer.rxq.data;
        let queue_data_pointer_rxq_data_slice = &mut *slice_from_raw_parts_mut(queue_data_pointer_rxq_data, RTE_MAX_QUEUES_PER_PORT as usize);
        queue_data_pointer_rxq_data_slice[queue_id as usize]
    };

    debug_assert!(queue_data != std::ptr::null_mut(), "Queue data pointer was null");

    let rx_pkt_burst = queue_data_pointer.rx_pkt_burst;

    debug_assert!(rx_pkt_burst.is_some());

    let buffer_len: u16 = rx_buffer.len() as u16;

    let rx_buffer_as_ptrs: *mut *mut rte_mbuf = unsafe { transmute(rx_buffer) };

    let mut number_received = unsafe {
        queue_data_pointer.rx_pkt_burst.unwrap()(queue_data, rx_buffer_as_ptrs, buffer_len)
    };

    // Callbacks
    {
        let callback_slice = unsafe {
            slice_from_raw_parts_mut(queue_data_pointer.rxq.clbk, RTE_MAX_QUEUES_PER_PORT as usize).as_mut().unwrap_unchecked()
        };
        let callback_ptr = AtomicPtr::from(callback_slice[queue_id as usize]);
        let callback = callback_ptr.load(std::sync::atomic::Ordering::Relaxed);
        if callback != std::ptr::null_mut() {
            number_received = unsafe {
                rte_eth_call_rx_callbacks(port_id, queue_id, rx_buffer_as_ptrs, number_received, buffer_len, callback)
            };
        }
    }

    return number_received;
}