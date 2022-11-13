use std::{marker::PhantomData, ptr::NonNull, cell::UnsafeCell};

use dpdk_sys::{
    rte_mempool, rte_mempool_cache, rte_mempool_create, rte_mempool_free, RTE_MAX_LCORE,
};

use crate::eal::{current_lcore_id, RteErrnoValue};


#[repr(transparent)]
pub struct Mempool<'mempool, T> {
    pool: *mut rte_mempool,
    _phantom: PhantomData<&'mempool T>,
}

impl<'mempool, T> Mempool<'mempool, T> {
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

        if pool.is_null() {
            Err(RteErrnoValue::most_recent())
        } else {
            Ok(Self {
                pool,
                _phantom: Default::default(),
            })
        }
    }

    unsafe fn get_default_cache(&self) -> Option<NonNull<rte_mempool_cache>> {
        if (*self.pool).cache_size == 0 {
            return None;
        }

        if current_lcore_id() >= RTE_MAX_LCORE as i32 {
            return None;
        }

        NonNull::new((*self.pool).local_cache)
    }
}

impl<'mempool, T> Drop for Mempool<'mempool, T> {
    fn drop(&mut self) {
        unsafe {
            rte_mempool_free(self.pool);
        }
    }
}

impl<'mempool, T> AsRef<rte_mempool> for Mempool<'mempool, T> {
    fn as_ref(&self) -> &rte_mempool {
        return unsafe { self.pool.as_ref() }.expect("Mempool pointer was null");
    }
}

impl<'mempool, T> AsMut<rte_mempool> for Mempool<'mempool, T> {
    fn as_mut(&mut self) -> &mut rte_mempool {
        return unsafe { self.pool.as_mut() }.expect("Mempool pointer was null");
    }
}

// pub struct MempoolGuard<'mempool, T> {
//     inner: &'mempool mut T,
//     pool: *mut rte_mempool
// }

// impl<'mempool, T> Drop for MempoolGuard<'mempool, T> {
//     fn drop(&mut self) {
//         unsafe {
//             rte_mempool_put(self.pool, self.inner as *mut T as *mut c_void);
//         }
//     }
// }

// impl<'mempool, T> Deref for MempoolGuard<'mempool, T> {
//     type Target = &'mempool mut T;

//     fn deref(&self) -> &Self::Target {
//         &self.inner
//     }
// }

// impl<'mempool, T> DerefMut for MempoolGuard<'mempool, T> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.inner
//     }
// }
