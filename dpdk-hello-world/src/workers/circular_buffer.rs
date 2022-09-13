use std::{cell::UnsafeCell, marker::PhantomData, mem::MaybeUninit, sync::Arc};

use dpdk::memory::allocator::{DPDKAllocator, DPDK_ALLOCATOR};

#[derive(Debug)]
struct RingBufferPositions {
    read: usize,
    written: usize,
}

type RingBufferMutex<T> = parking_lot::FairMutex<T>;

pub type RingBufferInterface<const SIZE: usize, T> = Arc<Box<RingBuffer<SIZE, T>, DPDKAllocator>>;

#[derive(Debug)]
pub struct RingBuffer<const SIZE: usize, T> {
    lock: RingBufferMutex<RingBufferPositions>,
    buffer: UnsafeCell<[T; SIZE]>,
}

unsafe impl<const SIZE: usize, T> Send for RingBuffer<SIZE, T> {}
unsafe impl<const SIZE: usize, T> Sync for RingBuffer<SIZE, T> {}

impl<const SIZE: usize, T> RingBuffer<SIZE, T> {
    pub fn new() -> Arc<Box<Self, DPDKAllocator>> {
        let mut buffer: Box<Self, DPDKAllocator> =
            unsafe { Box::new_uninit_in(DPDK_ALLOCATOR).assume_init() };
        // ensure the lock and position informat is in a good state
        let lock = buffer.lock.get_mut();
        lock.read = 0;
        lock.written = 0;
        drop(lock);
        return Arc::from(buffer);
    }

    /// Returns a mutable reference to the get buffer of this [`RingBuffer<SIZE, T>`].
    ///
    /// # Safety
    ///
    /// . May only be called when exclusive access to the buffer is held. This is typically via
    unsafe fn get_buffer(&self) -> &mut [T; SIZE] {
        unsafe { &mut *self.buffer.get() }
    }

    /// TODO: May break due to overflow
    pub fn write_next_blocking(&self, val: T) {
        loop {
            let mut guard = self.lock.lock();
            let difference = guard.written - guard.read;
            if difference < SIZE {
                let buffer = unsafe { self.get_buffer() };
                buffer[guard.written % SIZE] = val;
                guard.written += 1;
                return;
            }
        }
    }

    pub fn write_from_slice(&self, vals: &mut [T], mut count: usize) {
        while count > 0 {
            let mut guard = self.lock.lock();
            let difference = guard.written - guard.read;
            if difference < SIZE {
                let buffer = unsafe { self.get_buffer() };
                let num_moves = count.min(difference);
                for i in 0..count {
                    let value = vals.get_mut(i).unwrap();
                    buffer[guard.written % SIZE] = std::mem::replace(value, unsafe {
                        MaybeUninit::uninit().assume_init()
                    });
                    guard.written += 1;
                }
                count -= num_moves;
            }
        }
    }

    /// TODO: May break due to overflow
    pub fn read_next_blocking(&self) -> T {
        loop {
            let mut guard = self.lock.lock();
            if guard.written != guard.read {
                guard.read += 1;
                let buffer = unsafe { self.get_buffer() };
                let value = std::mem::replace(buffer.get_mut(guard.read % SIZE).unwrap(), unsafe {
                    MaybeUninit::uninit().assume_init()
                });
                return value;
            }
        }
    }
}

pub struct RingBufferIter<const SIZE: usize, T> {
    buffer: Arc<RingBuffer<SIZE, T>>,
    phantom: PhantomData<T>,
}

impl<const SIZE: usize, T> Iterator for RingBufferIter<SIZE, T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.buffer.read_next_blocking())
    }
}
