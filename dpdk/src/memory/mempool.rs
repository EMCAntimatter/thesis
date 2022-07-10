use std::marker::PhantomData;

use dpdk_sys::{rte_mempool, rte_mempool_create, rte_mempool_free};

use crate::eal::RteErrnoValue;

#[repr(transparent)]
pub struct MbufPool<T> {
    pool: *mut rte_mempool,
    _phantom: PhantomData<T>,
}

impl<T> MbufPool<T> {
    pub fn new(
        name: &'static str,
        num_elements: u32,
        cache_size: u32,
    ) -> Result<MbufPool<T>, RteErrnoValue> {
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

impl<T> Drop for MbufPool<T> {
    fn drop(&mut self) {
        unsafe {
            rte_mempool_free(self.pool);
        }
    }
}

impl<T> AsRef<rte_mempool> for MbufPool<T> {
    fn as_ref(&self) -> &rte_mempool {
        return unsafe { self.pool.as_ref() }.expect("Mempool pointer was null");
    }
}

impl<T> AsMut<rte_mempool> for MbufPool<T> {
    fn as_mut(&mut self) -> &mut rte_mempool {
        return unsafe { self.pool.as_mut() }.expect("Mempool pointer was null");
    }
}