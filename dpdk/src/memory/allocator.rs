use dpdk_sys::{rte_calloc, rte_free, rte_malloc, rte_realloc, rte_zmalloc};

use std::alloc::{GlobalAlloc, Layout};

struct DPDKAllocator {}

unsafe impl GlobalAlloc for DPDKAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();
        return rte_malloc(std::ptr::null(), size as dpdk_sys::size_t, align as u32) as *mut u8;
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        rte_free(ptr as *mut libc::c_void);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        return rte_zmalloc(std::ptr::null(), size as dpdk_sys::size_t, align as u32) as *mut u8;
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        return rte_realloc(
            ptr as *mut libc::c_void,
            new_size as dpdk_sys::size_t,
            align as u32,
        ) as *mut u8;
    }
}

// #[global_allocator]
// static ALLOCATOR: DPDKAllocator = DPDKAllocator {};
