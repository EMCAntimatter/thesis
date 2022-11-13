use core::slice;
use std::marker::PhantomData;

use dpdk_sys::{rte_mbuf, rte_pktmbuf_alloc, rte_pktmbuf_free_bulk};

use super::mempool::Mempool;

pub struct PktMbuf<'buff, T: ?Sized> {
    pub(crate) inner: *mut rte_mbuf,
    phantom: PhantomData<&'buff mut T>,
}

impl<'buff, T> PktMbuf<'buff, T> {
    /// Returns None on allocation failure
    pub fn new<'pool>(pool: &mut Mempool<'pool, T>) -> Option<Self>
    where
        'buff: 'pool,
    {
        let inner: *mut rte_mbuf = unsafe { rte_pktmbuf_alloc(pool.as_mut()) };
        if inner.is_null() {
            None
        } else {
            Some(Self {
                inner,
                phantom: Default::default(),
            })
        }
    }

    pub fn data(&self) -> &T {
        unsafe {
            let addr = (*self.inner).buf_addr.add((*self.inner).data_off as usize);
            &(*(addr as *const T))
        }
    }

    pub fn data_mut(&mut self) -> &mut T {
        unsafe {
            let addr = (*self.inner).buf_addr.add((*self.inner).data_off as usize);
            &mut (*(addr as *mut T))
        }
    }

    pub fn data_as_byte_slice(&self) -> &[u8] {
        unsafe {
            let addr = (*self.inner).buf_addr.add((*self.inner).data_off as usize);
            slice::from_raw_parts(addr as *mut u8, (*self.inner).data_len as usize)
        }
    }

    pub(crate) fn from_mbuf(mbuf: *mut rte_mbuf) -> Self {
        PktMbuf {
            inner: mbuf,
            phantom: Default::default(),
        }
    }
}

impl<'buff, T: ?Sized> Drop for PktMbuf<'buff, T> {
    fn drop(&mut self) {
        unsafe {
            let self_ptr_buf = [self.inner].as_mut_ptr();
            rte_pktmbuf_free_bulk(self_ptr_buf, 1);
        }
    }
}

impl<'buff, T: ?Sized> From<&'buff mut rte_mbuf> for PktMbuf<'buff, T> {
    fn from(buf: &'buff mut rte_mbuf) -> Self {
        Self {
            inner: buf,
            phantom: PhantomData::default(),
        }
    }
}

impl<T: ?Sized> From<*mut rte_mbuf> for PktMbuf<'static, T> {
    fn from(buf: *mut rte_mbuf) -> Self {
        Self {
            inner: buf,
            phantom: PhantomData::default(),
        }
    }
}
