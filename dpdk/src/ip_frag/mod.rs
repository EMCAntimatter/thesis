use std::mem::MaybeUninit;

use dpdk_sys::{
    rte_ip_frag_death_row, rte_ip_frag_free_death_row, rte_ip_frag_table_create,
    rte_ip_frag_table_del_expired_entries, rte_ip_frag_table_destroy, rte_ip_frag_tbl,
    rte_ipv4_frag_reassemble_packet, rte_ipv4_hdr,
};

use crate::memory::mbuf::PktMbuf;

pub struct IpFragTable {
    inner: *mut rte_ip_frag_tbl,
}

impl IpFragTable {
    /// Returns None on memory allocation failure
    pub fn new(
        num_buckets: u32,
        entries_per_bucket: u32,
        max_entries: u32,
        max_ttl_cycles: u64,
    ) -> Option<Self> {
        debug_assert_ne!(entries_per_bucket, 0);
        debug_assert_eq!(entries_per_bucket.count_ones(), 1); // is a power of two
        debug_assert!(max_entries <= num_buckets * entries_per_bucket);
        let ptr = unsafe {
            rte_ip_frag_table_create(
                num_buckets,
                entries_per_bucket,
                max_entries,
                max_ttl_cycles,
                0,
            )
        };
        if !ptr.is_null() {
            Some(IpFragTable { inner: ptr })
        } else {
            None
        }
    }

    pub fn delete_expired_entries(
        &mut self,
        expired_entry_storage: &mut StaleEntryBuffer,
        timestamp: u64,
    ) {
        unsafe {
            rte_ip_frag_table_del_expired_entries(
                self.inner,
                &mut expired_entry_storage.inner,
                timestamp,
            )
        }
    }

    /// Returns None if not all fragments have come back yet.
    pub fn ipv4_reassemble_packet<'a>(
        &mut self,
        stale_entries: &mut StaleEntryBuffer,
        fragment: &'a mut PktMbuf<rte_ipv4_hdr>,
        timestamp: u64,
    ) -> Option<PktMbuf<'a, rte_ipv4_hdr>> {
        let ipv4_header = fragment.data();
        let reassembled = unsafe {
            rte_ipv4_frag_reassemble_packet(
                self.inner,
                &mut stale_entries.inner,
                fragment.inner,
                timestamp,
                ipv4_header,
            )
        };
        if reassembled.is_null() {
            None
        } else {
            Some(PktMbuf::from_mbuf(reassembled))
        }
    }
}

impl Drop for IpFragTable {
    fn drop(&mut self) {
        unsafe { rte_ip_frag_table_destroy(self.inner) }
    }
}

pub struct StaleEntryBuffer {
    inner: rte_ip_frag_death_row,
}

impl StaleEntryBuffer {
    pub fn new() -> StaleEntryBuffer {
        StaleEntryBuffer {
            inner: rte_ip_frag_death_row {
                cnt: 0,
                row: unsafe { MaybeUninit::zeroed().assume_init() },
            },
        }
    }
}

impl Default for StaleEntryBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for StaleEntryBuffer {
    fn drop(&mut self) {
        unsafe { rte_ip_frag_free_death_row(&mut self.inner, 1) }
    }
}
