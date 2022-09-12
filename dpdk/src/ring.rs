use std::{sync::Arc, marker::PhantomData};

use dpdk_sys;

use crate::{eal::RteErrnoValue, util::str_to_c_string};

pub enum RingType {
    Single,
    MultipleRTS,
    MultipleHTS,
}

pub trait RteRingProducerHandle<'ring_lifetime, T> where T: 'ring_lifetime, Self: 'ring_lifetime {
    fn enqueue_bulk(&'ring_lifetime mut self, objects: impl AsRef<[&'ring_lifetime mut T]>);
    fn enqueue(&'ring_lifetime mut self, objects: impl AsRef<[&'ring_lifetime mut T]>);
}

pub trait RteRingConsumerHandle<'ring_lifetime, T> where T: 'ring_lifetime, Self: 'ring_lifetime {
    fn dequeue_bulk(&'ring_lifetime mut self, objects: &mut [&mut T], n: u32);
    fn dequeue(&'ring_lifetime mut self, objects: &mut [&mut T]);
}


pub struct RteRingSPHandle<T> {
    ring: Arc<RteRing<T>>,
    _phantom: PhantomData<*mut dpdk_sys::rte_ring>
}

#[derive(Clone)]
pub struct RteRingMPHandle<T> {
    ring: Arc<RteRing<T>>,
}

pub struct RteRingSCHandle<T> {
    ring: Arc<RteRing<T>>,
    _phantom: PhantomData<*mut dpdk_sys::rte_ring>
}

#[derive(Clone)]
pub struct RteRingMCHandle<T> {
    ring: Arc<RteRing<T>>
}


pub struct RteRing<T> {
    inner: *mut dpdk_sys::rte_ring,
    _phantom: PhantomData<*mut T>
}

impl<T> RteRing<T> {
    pub fn new(
        name: impl AsRef<str>,
        size: u32,
        numa_socket_id: i32,
        producer_type: RingType,
        consumer_type: RingType,
    ) -> Result<Arc<Self>, RteErrnoValue> {
        let str = str_to_c_string(name.as_ref());
        let str = Box::new(str);
        let str = Box::leak(str);
        let str = str.as_c_str().as_ptr();
        let mut flags = dpdk_sys::RING_F_EXACT_SZ;

        flags |= match producer_type {
            RingType::Single => dpdk_sys::RING_F_SP_ENQ,
            RingType::MultipleRTS => dpdk_sys::RING_F_MP_RTS_ENQ,
            RingType::MultipleHTS => dpdk_sys::RING_F_MP_HTS_ENQ,
        };

        flags |= match consumer_type {
            RingType::Single => dpdk_sys::RING_F_SC_DEQ,
            RingType::MultipleRTS => dpdk_sys::RING_F_MC_RTS_DEQ,
            RingType::MultipleHTS => dpdk_sys::RING_F_MC_HTS_DEQ,
        };

        let ptr = unsafe { dpdk_sys::rte_ring_create(str, size, numa_socket_id, flags) };

        if ptr.is_null() {
            Err(RteErrnoValue::most_recent())
        } else {
            let handle = Arc::new(Self { inner: ptr, _phantom: Default::default() });
            Ok(handle)
        }
    }
}

impl<T> Drop for RteRing<T> {
    fn drop(&mut self) {
        unsafe {
            dpdk_sys::rte_ring_free(self.inner)
        }
    }
}
