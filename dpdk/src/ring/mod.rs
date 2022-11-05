use std::{alloc::Layout, os::raw::c_void, ptr::NonNull, sync::Arc};

use crate::{
    eal::RteErrnoValue,
    memory::allocator::{DPDKBox, DPDK_ALLOCATOR},
};

/// DPDK Ring implementation
pub mod dpdk_ring;
///
/// Contains extern declarations for the inline declarations of rte_ring
///
mod externs;
/// Rust ring implementation
pub mod rusty_ring;

pub type RingObjectTable<'ring_lifetime, T> = DPDKBox<[T]>;

impl<'ring_lifetime, T: 'ring_lifetime> RingObjectTable<'ring_lifetime, T> {
    pub fn new(size: u32) -> Self {
        let slice: NonNull<[T]> =
            unsafe { DPDK_ALLOCATOR.allocate_array_zeroed(size as usize).unwrap() };
        DPDKBox::from_non_null(slice)
    }

    pub fn new_const_size<const SIZE: u32>() -> DPDKBox<[T; SIZE as usize]> {
        unsafe { DPDKBox::new_zeroed() }
    }

    /// Does the conversion in-place
    /// Does NOT leak the memory.
    fn to_dpdk_object_table(&self) -> (*const c_void, u32)
    where
        T: 'ring_lifetime,
    {
        debug_assert_eq!(Layout::new::<Option<T>>(), Layout::new::<*mut c_void>());
        let ptr = self.as_ptr();
        let array: *const c_void = unsafe { std::mem::transmute(ptr) };
        debug_assert_eq!(Layout::for_value(&ptr), Layout::for_value(&array));
        (array, self.len() as u32)
    }
}

pub enum DequeueFailureState {
    Timeout,
}

pub trait RingDataRequirements<'ring_lifetime>: 'ring_lifetime + Send + Sync + Copy {}

pub trait Ring<'ring_lifetime, T>
where
    T: RingDataRequirements<'ring_lifetime>,
{
    fn new(name: impl AsRef<str>, size: u32) -> Result<Arc<Self>, RteErrnoValue>;

    /// # Description
    ///
    /// Tries to insert `val` into the underlying ring buffer.
    ///
    /// # Return
    /// If the ring buffer is full, returns `Err(val)`. Otherwise,
    /// returns `()`
    fn try_enqueue(&self, val: T) -> Result<(), T>;

    fn enqueue(&self, val: T) {
        let mut to_insert = val;
        loop {
            match self.try_enqueue(to_insert) {
                Ok(()) => return,
                Err(val) => to_insert = val,
            }
        }
    }

    fn try_enqueue_from(
        &self,
        buffer: &mut RingObjectTable<'ring_lifetime, T>,
        insert_index: usize,
    ) -> u32;

    fn enqueue_from(&self, buffer: &mut RingObjectTable<'ring_lifetime, T>) {
        let mut push_index = 0;
        let buffer_len = buffer.len();
        while push_index < buffer_len {
            push_index += self.try_enqueue_from(buffer, push_index) as usize;
        }
    }

    fn try_dequeue(&self) -> Option<T>;

    fn dequeue<const MAX_CYCLES: u64>(&self) -> Result<Option<T>, DequeueFailureState> {
        for _ in 0..MAX_CYCLES {
            let val = self.try_dequeue();
            if val.is_some() {
                return Ok(val);
            }
        }
        Err(DequeueFailureState::Timeout)
    }

    fn try_dequeue_into(
        &self,
        buffer: &mut RingObjectTable<'ring_lifetime, T>,
        insert_index: usize,
    ) -> u32;

    fn dequeue_into<const MAX_CYCLES: u64>(&self, buffer: &mut RingObjectTable<'ring_lifetime, T>) {
        let mut push_index = 0;
        let buffer_len = buffer.len();
        while push_index < buffer_len {
            push_index += self.try_dequeue_into(buffer, push_index) as usize;
        }
    }
}

pub trait PeekGuard<'ring_lifetime, T>
where
    T: RingDataRequirements<'ring_lifetime>,
{
}

pub trait PeekableRing<'ring_lifetime, T>: Ring<'ring_lifetime, T>
where
    T: RingDataRequirements<'ring_lifetime>,
{
    type Guard
    where
        <Self as PeekableRing<'ring_lifetime, T>>::Guard: PeekGuard<'ring_lifetime, T>;

    fn peek(&self) -> Self::Guard
    where
        <Self as PeekableRing<'ring_lifetime, T>>::Guard: PeekGuard<'ring_lifetime, T>;
}
