use std::ptr::slice_from_raw_parts_mut;

use dpdk_sys::{rte_event, rte_event_fp_ops, RTE_EVENT_MAX_PORTS_PER_DEV};

use super::{EventDeviceId, EventPortId};

#[inline(always)]
pub fn enqueue_new_events(
    eventdev_id: EventDeviceId,
    port_id: EventPortId,
    events: &mut [rte_event],
    length: usize,
) -> u16 {
    // if length > 0 {
        unsafe {
            let fp_ops = rte_event_fp_ops[eventdev_id as usize];
            let data_slice =
                slice_from_raw_parts_mut(fp_ops.data, RTE_EVENT_MAX_PORTS_PER_DEV as usize)
                    .as_mut()
                    .unwrap();
            let port = data_slice[port_id as usize];
            let event_ptr = &events[0];
            if length == 1 {
                return fp_ops.enqueue.unwrap_unchecked()(port, event_ptr);
            } else {
                return fp_ops.enqueue_burst.unwrap_unchecked()(port, event_ptr, length as u16);
            }
        }
    // } else {
        // 0
    // }
}

#[inline(always)]
pub fn enqueue_foward_events(
    eventdev_id: EventDeviceId,
    port_id: EventPortId,
    events: &mut [rte_event],
    length: usize,
) -> u16 {
    unsafe {
        let fp_ops = rte_event_fp_ops[eventdev_id as usize];
        let data_slice =
            slice_from_raw_parts_mut(fp_ops.data, RTE_EVENT_MAX_PORTS_PER_DEV as usize)
                .as_mut()
                .unwrap();
        let port = data_slice[port_id as usize];
        let event_ptr = &events[0];
        if length == 1 {
            return fp_ops.enqueue.unwrap_unchecked()(port, event_ptr);
        } else {
            return fp_ops.enqueue_forward_burst.unwrap_unchecked()(port, event_ptr, length as u16);
        }
    }
}

#[inline(always)]
pub fn enqueue_tx_adapter_events(
    eventdev_id: EventDeviceId,
    port_id: EventPortId,
    events: &mut [rte_event],
    length: usize,
) -> u16 {
    unsafe {
        let fp_ops = rte_event_fp_ops[eventdev_id as usize];
        let data_slice =
            slice_from_raw_parts_mut(fp_ops.data, RTE_EVENT_MAX_PORTS_PER_DEV as usize)
                .as_mut()
                .unwrap();
        let port = data_slice[port_id as usize];
        // let event_ptr = &events[0];
        return fp_ops.txa_enqueue.unwrap_unchecked()(port, events.as_mut_ptr(), length as u16);
    }
}

#[inline(always)]
pub fn dequeue_events(
    eventdev_id: EventDeviceId,
    port_id: EventPortId,
    events: &mut [rte_event],
    timeout: u64,
) -> u16 {
    unsafe {
        let fp_ops = rte_event_fp_ops[eventdev_id as usize];
        let data_slice =
            slice_from_raw_parts_mut(fp_ops.data, RTE_EVENT_MAX_PORTS_PER_DEV as usize)
                .as_mut()
                .unwrap();
        let port = data_slice[port_id as usize];
        if events.len() == 1 {
            return fp_ops.dequeue.unwrap_unchecked()(port, events.as_mut_ptr(), timeout);
        } else {
            return fp_ops.dequeue_burst.unwrap_unchecked()(
                port,
                events.as_mut_ptr(),
                events.len() as u16,
                timeout,
            );
        }
    }
}
