use std::marker::PhantomData;

use dpdk_sys::{rte_mempool, rte_mempool_create, rte_mempool_free};

use crate::eal::RteErrnoValue;

#[repr(transparent)]
pub struct MbufPool<'mempool, T> {
    pool: *mut rte_mempool,
    _phantom: PhantomData<&'mempool T>,
}

impl<'mempool, T> MbufPool<'mempool, T> {
    pub fn new(
        name: &'static str,
        num_elements: u32,
        cache_size: u32,
    ) -> Result<Self, RteErrnoValue> {
        let c_name = name.as_ptr() as *const i8;
        let pool = unsafe {
            rte_mempool_create(
                c_name,
                num_elements,
                std::mem::size_of::<T>() as u32,
                cache_size,
                0,
                None,
                std::ptr::null_mut(),
                None,
                std::ptr::null_mut(),
                dpdk_sys::rte_socket_id() as i32,
                0,
            )
        };

        if pool == std::ptr::null_mut() {
            Err(RteErrnoValue::most_recent())
        } else {
            Ok(Self {
                pool,
                _phantom: Default::default(),
            })
        }
    }
}

impl<'mempool, T> Drop for MbufPool<'mempool, T> {
    fn drop(&mut self) {
        unsafe {
            rte_mempool_free(self.pool);
        }
    }
}

impl<'mempool, T> AsRef<rte_mempool> for MbufPool<'mempool, T> {
    fn as_ref(&self) -> &rte_mempool {
        return unsafe { self.pool.as_ref() }.expect("Mempool pointer was null");
    }
}

impl<'mempool, T> AsMut<rte_mempool> for MbufPool<'mempool, T> {
    fn as_mut(&mut self) -> &mut rte_mempool {
        return unsafe { self.pool.as_mut() }.expect("Mempool pointer was null");
    }
}