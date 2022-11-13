use std::ptr::NonNull;

use dpdk::{
    device::eth::dev::{EthdevPortId, EventQueueId},
    memory::allocator::{DPDKAllocator, DPDK_ALLOCATOR},
    raw::rte_mbuf,
};

use crate::{ETHDEV_PORT_ID, ETHDEV_QUEUE_ID};

use super::pipeline::{SpscConsumerChannelHandle, SpscProducerChannelHandle};

// use super::circular_buffer::RingBufferInterface;

// pub fn read_packets_from_nic_port_0_into_ring<const SIZE: usize>(
//     output_ring: RingBufferInterface<SIZE, &mut rte_mbuf>,
// ) {
//     let mut buffer = unsafe {
//         Box::<[&mut rte_mbuf; SIZE], DPDKAllocator>::new_uninit_in(DPDK_ALLOCATOR).assume_init()
//     };
//     loop {
//         let count = dpdk::device::eth::rx::receive_burst(0, 0, buffer.as_mut()) as usize;
//         buffer.iter_mut().take(count).for_each(|i| {
//             let reference = swap_with_uninit!(i);
//             output_ring.write_next_blocking(reference);
//         });
//     }
// }

pub fn read_from_nic_port_into_buffer<
    const BUFFER_SIZE: usize,
    const ETHDEV_PORT_ID: EthdevPortId,
    const ETHDEV_QUEUE_ID: EventQueueId,
>(
    buf: &mut [Option<NonNull<rte_mbuf>>; BUFFER_SIZE],
) -> usize {
    return dpdk::device::eth::rx::receive_burst(ETHDEV_PORT_ID, ETHDEV_QUEUE_ID, buf) as usize;
}

pub fn read_packets_from_nic_port_into_channel<
    const BUFFER_SIZE: usize,
    const ETHDEV_PORT_ID: EthdevPortId,
    const ETHDEV_QUEUE_ID: EventQueueId,
>(
    mut output_channel: SpscProducerChannelHandle<&'static mut rte_mbuf>,
) {
    let mut buffer = unsafe {
        Box::<[Option<NonNull<rte_mbuf>>; BUFFER_SIZE], DPDKAllocator>::new_uninit_in(
            DPDK_ALLOCATOR,
        )
        .assume_init()
    };
    loop {
        let mut written_to_channel = 0;
        let count = read_from_nic_port_into_buffer::<BUFFER_SIZE, ETHDEV_PORT_ID, ETHDEV_QUEUE_ID>(
            &mut buffer,
        );
        while written_to_channel < count {
            let remaining = count - written_to_channel;
            let to_write = output_channel.slots().min(remaining);
            let mut chunk = output_channel.write_chunk_uninit(to_write).unwrap();
            let (first, second) = chunk.as_mut_slices();
            written_to_channel += unsafe {
                // Fill first
                let first_from_ptr = buffer.as_ptr().add(written_to_channel);
                let first_to_ptr = std::mem::transmute(first.as_mut_ptr());
                std::ptr::copy_nonoverlapping(first_from_ptr, first_to_ptr, first.len());
                let second_from_ptr = first_from_ptr.add(first.len());
                let second_to_ptr = std::mem::transmute(first.as_mut_ptr());
                std::ptr::copy_nonoverlapping(second_from_ptr, second_to_ptr, second.len());

                let written = first.len() + second.len();
                chunk.commit(written);
                written
            };
        }
    }
}

pub fn write_packets_to_nic_port_0(
    mut input_channel: SpscConsumerChannelHandle<&'static mut rte_mbuf>,
) -> Result<(), anyhow::Error> {
    loop {
        let num_to_read = input_channel.slots().min(u16::MAX as usize);
        let chunk = input_channel.read_chunk(num_to_read).unwrap();
        let (first, second) = chunk.as_slices();

        for chunk in [first, second] {
            let mut buff = chunk;
            let mut sent = 0;
            while sent < chunk.len() {
                let num_tx: usize = dpdk::device::eth::tx::send_burst(
                    ETHDEV_PORT_ID,
                    ETHDEV_QUEUE_ID,
                    buff,
                    buff.len().min(u16::MAX as usize) as u16,
                ) as usize;
                buff = &buff[num_tx..buff.len()];
                sent += num_tx;
            }
        }
    }
}
