use dpdk_sys::{
    rte_mempool, rte_mempool_free, rte_pktmbuf_pool_create, rte_socket_id,
    RTE_MBUF_DEFAULT_BUF_SIZE,
};

use crate::eal::RteErrnoValue;

#[repr(transparent)]
pub struct PktMbufPool {
    pool: *mut rte_mempool,
}

impl PktMbufPool {
    pub fn new(
        name: &'static str,
        num_elements: u32,
        cache_size: u32,
    ) -> Result<Self, RteErrnoValue> {
        let c_name = name.as_ptr() as *const i8;
        assert!(name.contains('\0'));
        let pool = unsafe {
            rte_pktmbuf_pool_create(
                c_name,
                num_elements,
                cache_size,
                0,
                RTE_MBUF_DEFAULT_BUF_SIZE as u16,
                rte_socket_id() as i32,
            )
        };

        if pool.is_null() {
            Err(RteErrnoValue::most_recent())
        } else {
            Ok(Self { pool })
        }
    }
}

impl Drop for PktMbufPool {
    fn drop(&mut self) {
        unsafe { rte_mempool_free(self.pool) }
    }
}

impl AsRef<rte_mempool> for PktMbufPool {
    fn as_ref(&self) -> &rte_mempool {
        return unsafe { self.pool.as_ref() }.expect("Mempool pointer was null");
    }
}

impl AsMut<rte_mempool> for PktMbufPool {
    fn as_mut(&mut self) -> &mut rte_mempool {
        return unsafe { self.pool.as_mut() }.expect("Mempool pointer was null");
    }
}
