use std::mem::MaybeUninit;

use dpdk::{
    memory::allocator::{DPDKAllocator, DPDK_ALLOCATOR},
    raw::rte_mbuf,
};
use crossbeam_channel::{Receiver, Sender};

use crate::{swap_with_uninit, ETHDEV_PORT_ID, ETHDEV_QUEUE_ID};

use super::circular_buffer::RingBufferInterface;

pub fn read_packets_from_nic_port_0_into_ring<const SIZE: usize>(
    output_ring: RingBufferInterface<SIZE, &mut rte_mbuf>,
) {
    let mut buffer = unsafe {
        Box::<[&mut rte_mbuf; SIZE], DPDKAllocator>::new_uninit_in(DPDK_ALLOCATOR).assume_init()
    };
    loop {
        let count = dpdk::device::eth::rx::receive_burst(0, 0, buffer.as_mut()) as usize;
        buffer.iter_mut().take(count).for_each(|i| {
            let reference = swap_with_uninit!(i);
            output_ring.write_next_blocking(reference);
        });
    }
}

pub fn read_packets_from_nic_port_0_into_channel<const SIZE: usize>(
    output_channel: Sender<&'static mut rte_mbuf>,
) {
    let mut buffer = unsafe {
        Box::<[&mut rte_mbuf; SIZE], DPDKAllocator>::new_uninit_in(DPDK_ALLOCATOR).assume_init()
    };
    loop {
        let count =
            dpdk::device::eth::rx::receive_burst(ETHDEV_PORT_ID, ETHDEV_QUEUE_ID, buffer.as_mut())
                as usize;
        for i in 0..count {
            output_channel.send(swap_with_uninit!(unsafe { buffer.get_unchecked_mut(i) })).unwrap();
        }
    }
}

pub fn write_packets_to_nic_port_0<const SIZE: usize>(
    input_channel: Receiver<&'static mut rte_mbuf>,
) {
    let mut buffer = unsafe {
        Box::<[&mut rte_mbuf; SIZE], DPDKAllocator>::new_uninit_in(DPDK_ALLOCATOR).assume_init()
    };
    let mut count: u16 = 0;
    loop {
        let mut iter = input_channel.try_iter();
        while let Some(val) = iter.next() {
            if (count as usize) < SIZE {
                buffer[count as usize] = val;
                count += 1;
            } else {
                while count != 0 {
                    let sent = dpdk::device::eth::tx::send_burst(
                        ETHDEV_PORT_ID,
                        ETHDEV_QUEUE_ID,
                        &mut buffer,
                        count as u16,
                    );
                    debug_assert!(
                        count >= sent,
                        "More packets sent than were requested to be sent"
                    );
                    count -= sent;
                }
            }
        }
    }
}
