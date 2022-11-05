use dpdk_sys::{rte_ring, rte_ring_zc_data};

extern "C" {
    /// # Description
    ///
    /// Enqueues exactly one object.
    ///
    /// # Return Value
    ///
    /// * 0: Success; objects enqueued
    /// * -ENOBUFS: No enough room in the ring to enqueue; no object is enqueued
    pub fn rte_ring_enqueue(ring: *mut rte_ring, obj: *mut ::std::os::raw::c_void) -> i32;
    pub fn rte_ring_enqueue_elem(
        ring: *mut rte_ring,
        obj: *const ::std::os::raw::c_void,
        esize: u32,
    ) -> i32;
    /// Enqueues either 0 or exactly n objects, returns the number of objects enqueued
    pub fn rte_ring_enqueue_bulk(
        ring: *mut rte_ring,
        object_table: *const *mut ::std::os::raw::c_void,
        n: u32,
        free_space: *mut u32,
    ) -> u32;
    /// Enqueues as many objects as possible, returning the number of objects enqueued
    pub fn rte_ring_enqueue_burst(
        ring: *mut rte_ring,
        object_table: *const *mut ::std::os::raw::c_void,
        n: u32,
        free_space: *mut u32,
    ) -> u32;
    pub fn rte_ring_enqueue_burst_elem(
        ring: *mut rte_ring,
        object_table: *const ::std::os::raw::c_void,
        esize: u32,
        n: u32,
        free_space: *mut u32,
    ) -> u32;
    pub fn rte_ring_enqueue_zc_bulk_elem_start(
        ring: *mut rte_ring,
        esize: u32,
        n: u32,
        zcd: *const rte_ring_zc_data,
        free_space: *const u32,
    ) -> u32;
    pub fn rte_ring_enqueue_zc_elem_finish(ring: *mut rte_ring, n: u32);
    /// # Description
    ///
    /// Dequeues exactly one object.
    ///
    /// # Return Value
    ///
    /// * 0: Success; object dequeued
    /// * ENOENT: Not enough entries in the ring to dequeue, no object is dequeued.
    pub fn rte_ring_dequeue(ring: *mut rte_ring, obj: *const *mut ::std::os::raw::c_void) -> i32;
    /// Dequeues as many objects as possible, returning the number of objects dequeued
    pub fn rte_ring_dequeue_burst(
        ring: *mut rte_ring,
        object_table: *const *mut ::std::os::raw::c_void,
        n: u32,
        avaliable_space: *mut u32,
    ) -> u32;

    pub fn rte_ring_dequeue_elem(
        ring: *mut rte_ring,
        obj: *const ::std::os::raw::c_void,
        esize: u32,
    ) -> i32;
    pub fn rte_ring_dequeue_burst_elem(
        ring: *mut rte_ring,
        object_table: *const ::std::os::raw::c_void,
        esize: u32,
        n: u32,
        avaliable_space: *mut u32,
    ) -> u32;
    pub fn rte_ring_dequeue_zc_bulk_elem_start(
        ring: *mut rte_ring,
        esize: u32,
        n: u32,
        zcd: *const rte_ring_zc_data,
        avaliable_space: *const u32,
    ) -> u32;
    pub fn rte_ring_dequeue_zc_elem_finish(ring: *mut rte_ring, n: u32);
}
