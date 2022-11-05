use super::{externs, PeekGuard, Ring, RingDataRequirements, RingObjectTable};

use std::{marker::PhantomData, mem::MaybeUninit, os::raw::c_void, ptr::NonNull, sync::Arc};

use dpdk_sys::{self, rte_ring, rte_ring_zc_data, ENOBUFS};

use crate::{eal::RteErrnoValue, util::str_to_c_string};

pub struct DPDKRing<'ring_lifetime, T: 'ring_lifetime> {
    inner: *mut dpdk_sys::rte_ring,
    _phantom: PhantomData<&'ring_lifetime mut T>,
}

impl<'ring_lifetime, T: 'ring_lifetime> DPDKRing<'ring_lifetime, T> {}

impl<'ring_lifetime, T> Ring<'ring_lifetime, T> for DPDKRing<'ring_lifetime, T>
where
    T: RingDataRequirements<'ring_lifetime>,
{
    fn new(name: impl AsRef<str>, size: u32) -> Result<Arc<Self>, RteErrnoValue> {
        let str = str_to_c_string(name.as_ref());
        let str = Box::new(str);
        let str = Box::leak(str);
        let str = str.as_c_str().as_ptr();
        let mut flags = dpdk_sys::RING_F_EXACT_SZ;

        // Only enable this for overcommitted systems
        flags |= dpdk_sys::RING_F_MP_HTS_ENQ;
        flags |= dpdk_sys::RING_F_MC_HTS_DEQ;

        let t_size = std::mem::size_of::<T>();
        debug_assert_eq!(t_size % 4, 0);

        let ptr = unsafe { dpdk_sys::rte_ring_create_elem(str, t_size as u32, size, 0, flags) };

        if ptr.is_null() {
            Err(RteErrnoValue::most_recent())
        } else {
            let handle = Arc::new(Self {
                inner: ptr,
                _phantom: Default::default(),
            });
            Ok(handle)
        }
    }

    fn try_enqueue(&self, val: T) -> Result<(), T> {
        let ptr = (&val) as *const T as *const c_void;
        // It has been copied into the ring
        let ret = unsafe {
            externs::rte_ring_enqueue_elem(self.inner, ptr, std::mem::size_of::<T>() as u32)
        };
        match -ret as u32 {
            0 => Ok(()),
            ENOBUFS => Err(val),
            unknown => {
                unimplemented!("Unimplemented error value {unknown}")
            }
        }
    }

    /// returns the number inserted
    fn try_enqueue_from(
        &self,
        buffer: &mut RingObjectTable<'ring_lifetime, T>,
        insert_index: usize,
    ) -> u32 {
        debug_assert!(insert_index < buffer.len());
        let (mut c_array_to_insert, n) = buffer.to_dpdk_object_table();
        c_array_to_insert = unsafe { c_array_to_insert.add(insert_index) };
        let mut space_left_in_table: u32 = 0;

        unsafe {
            externs::rte_ring_enqueue_burst_elem(
                self.inner,
                c_array_to_insert,
                std::mem::size_of::<T>() as u32,
                n,
                &mut space_left_in_table,
            )
        }
    }

    fn try_dequeue(&self) -> Option<T> {
        #[allow(clippy::uninit_assumed_init)]
        let obj: T = unsafe { MaybeUninit::uninit().assume_init() };
        let ptr: *const c_void = (&obj) as *const T as *const c_void;
        let ret = unsafe {
            externs::rte_ring_dequeue_elem(self.inner, ptr, std::mem::size_of::<T>() as u32)
        };

        match -ret as u32 {
            0 => Some(obj),
            ENOBUFS => None,
            unknown => {
                panic!("Unknown error type: {unknown}")
            }
        }
    }

    /// Takes a buffer as a reference to allow for re-use
    /// returns the number inserted if not all of them were inserted
    fn try_dequeue_into(
        &self,
        buffer: &mut RingObjectTable<'ring_lifetime, T>,
        insert_index: usize,
    ) -> u32 {
        debug_assert!(insert_index < buffer.len());
        let (mut object_table, buffer_len) = buffer.to_dpdk_object_table();
        object_table = unsafe { object_table.add(insert_index) };

        unsafe {
            externs::rte_ring_dequeue_burst_elem(
                self.inner,
                object_table,
                std::mem::size_of::<T>() as u32,
                buffer_len,
                std::ptr::null_mut(),
            )
        }
    }

    fn dequeue_into<const MAX_CYCLES: u64>(&self, buffer: &mut RingObjectTable<'ring_lifetime, T>) {
        let mut push_index = 0;
        let buffer_len = buffer.len();
        while push_index < buffer_len {
            push_index += self.try_dequeue_into(buffer, push_index) as usize;
        }
    }
}

unsafe impl<'ring_lifetime, T> Send for DPDKRing<'ring_lifetime, T> {}
unsafe impl<'ring_lifetime, T> Sync for DPDKRing<'ring_lifetime, T> {}

impl<'ring_lifetime, T> Drop for DPDKRing<'ring_lifetime, T> {
    fn drop(&mut self) {
        unsafe { dpdk_sys::rte_ring_free(self.inner) }
    }
}

pub struct DPDKPeekGuard<'ring_lifetime, const BUFFER_SIZE: u32, T>
where
    T: RingDataRequirements<'ring_lifetime>,
{
    ring: *mut rte_ring,
    zcd: rte_ring_zc_data,
    num_entries: u32,
    _phantom: PhantomData<&'ring_lifetime [T]>,
}

impl<'ring_lifetime, const BUFFER_SIZE: u32, T> DPDKPeekGuard<'ring_lifetime, BUFFER_SIZE, T>
where
    T: RingDataRequirements<'ring_lifetime>,
{
    pub fn new(ring: NonNull<rte_ring>) -> Self {
        let mut s = Self {
            ring: ring.as_ptr(),
            zcd: rte_ring_zc_data {
                ptr1: std::ptr::null_mut(),
                ptr2: std::ptr::null_mut(),
                n1: 0,
            },
            num_entries: 0,
            _phantom: PhantomData::default(),
        };
        s.num_entries = unsafe {
            externs::rte_ring_dequeue_zc_bulk_elem_start(
                ring.as_ptr(),
                std::mem::size_of::<T>() as u32,
                BUFFER_SIZE,
                &s.zcd,
                std::ptr::null_mut(),
            )
        };
        s
    }

    pub fn take_n(self, n: u32) {
        unsafe { externs::rte_ring_dequeue_zc_elem_finish(self.ring, n) }
    }
}

impl<'ring_lifetime, const BUFFER_SIZE: u32, T> PeekGuard<'ring_lifetime, T>
    for DPDKPeekGuard<'ring_lifetime, BUFFER_SIZE, T>
where
    T: RingDataRequirements<'ring_lifetime>,
{
}

impl<'ring_lifetime, const BUFFER_SIZE: u32, T> Drop
    for DPDKPeekGuard<'ring_lifetime, BUFFER_SIZE, T>
where
    T: RingDataRequirements<'ring_lifetime>,
{
    fn drop(&mut self) {
        unsafe { externs::rte_ring_dequeue_zc_elem_finish(self.ring, 0) }
    }
}
