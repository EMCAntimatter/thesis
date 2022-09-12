use std::marker::PhantomData;

use dpdk_sys::{rte_mbuf, rte_pktmbuf_alloc};


use super::mempool::MbufPool;

pub struct PktMbuf<'buff, T> {
    pub(crate) inner: *mut rte_mbuf,
    phantom: PhantomData<&'buff T>,
}

impl<'buff, T> PktMbuf<'buff, T> {
    /// Returns None on allocation failure
    pub fn new<'pool>(pool: &mut MbufPool<'pool, T>) -> Option<Self>
    where
        'buff: 'pool,
    {
        let inner: *mut rte_mbuf = unsafe { rte_pktmbuf_alloc(pool.as_mut()) };
        if inner == std::ptr::null_mut() {
            None
        } else {
            Some(Self {
                inner,
                phantom: Default::default(),
            })
        }
    }

    pub fn data<'a>(&'a self) -> &'a T {
        unsafe {
            let addr = (*self.inner).buf_addr.add((*self.inner).data_off as usize);
            &(*(addr as *const T))
        }
    }

    pub fn data_mut<'a>(&'a mut self) -> &'a mut T {
        unsafe {
            let addr = (*self.inner).buf_addr.add((*self.inner).data_off as usize);
            & mut(*(addr as *mut T))
        }
    }

    pub(crate) fn from_mbuf(mbuf: *mut rte_mbuf) -> Self  {
        PktMbuf { inner: mbuf, phantom: Default::default() }
    }
}

impl<'buff, T> Drop for PktMbuf<'buff, T> {
    fn drop(&mut self) {
        todo!()
    }
}
