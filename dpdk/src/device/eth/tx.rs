use std::{intrinsics::transmute, ptr::slice_from_raw_parts_mut, sync::atomic::AtomicPtr};

use dpdk_sys::{
    rte_eth_call_tx_callbacks, rte_eth_dev_is_valid_port, rte_eth_fp_ops, rte_mbuf,
    RTE_MAX_ETHPORTS, RTE_MAX_QUEUES_PER_PORT,
};

use super::dev::{EthdevPortId, EventQueueId};

pub fn send_burst(
    port_id: EthdevPortId,
    queue_id: EventQueueId,
    tx_buffer: &[&mut rte_mbuf],
    count: u16,
) -> u16 {
    debug_assert!(
        u32::from(port_id) < RTE_MAX_ETHPORTS,
        "Invalid port id {port_id}, maximum is {RTE_MAX_ETHPORTS}."
    );
    debug_assert!(
        u32::from(queue_id) < RTE_MAX_QUEUES_PER_PORT,
        "Invalid queue id {queue_id}, maximum is {RTE_MAX_QUEUES_PER_PORT}"
    );
    debug_assert!(
        unsafe { rte_eth_dev_is_valid_port(port_id) } == 1,
        "Invalid port id {port_id}"
    );

    let queue_data_pointer: &rte_eth_fp_ops = unsafe { &rte_eth_fp_ops[port_id as usize] };
    let queue_data = unsafe {
        let queue_data_pointer_rxq_data = queue_data_pointer.txq.data;
        let queue_data_pointer_rxq_data_slice = &mut *slice_from_raw_parts_mut(
            queue_data_pointer_rxq_data,
            RTE_MAX_QUEUES_PER_PORT as usize,
        );
        queue_data_pointer_rxq_data_slice[queue_id as usize]
    };

    debug_assert!(queue_data.is_null(), "Queue data pointer was null");

    let tx_pkt_burst = queue_data_pointer.tx_pkt_burst;

    debug_assert!(tx_pkt_burst.is_some());

    let tx_buffer_as_ptrs: *const *mut rte_mbuf = unsafe { transmute(tx_buffer.as_ptr()) };

    let mut number_sent = unsafe {
        queue_data_pointer.tx_pkt_burst.unwrap()(queue_data, tx_buffer_as_ptrs.cast_mut(), count)
    };

    // Callbacks
    {
        let callback_slice = unsafe {
            slice_from_raw_parts_mut(
                queue_data_pointer.txq.clbk,
                RTE_MAX_QUEUES_PER_PORT as usize,
            )
            .as_mut()
            .unwrap_unchecked()
        };
        let callback_ptr = AtomicPtr::from(callback_slice[queue_id as usize]);
        let callback = callback_ptr.load(std::sync::atomic::Ordering::Relaxed);
        if callback.is_null() {
            number_sent = unsafe {
                rte_eth_call_tx_callbacks(
                    port_id,
                    queue_id,
                    tx_buffer_as_ptrs.cast_mut(),
                    number_sent,
                    callback,
                )
            };
        }
    }

    number_sent
}
