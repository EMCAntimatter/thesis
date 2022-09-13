use dpdk_sys::{rte_free, rte_malloc, rte_realloc, rte_zmalloc};

use core::slice;
use std::alloc::{AllocError, Allocator, GlobalAlloc, Layout};

pub struct DPDKAllocator {}

unsafe impl GlobalAlloc for DPDKAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();
        return rte_malloc(std::ptr::null(), size as dpdk_sys::size_t, align as u32) as *mut u8;
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        rte_free(ptr as *mut libc::c_void);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        return rte_zmalloc(std::ptr::null(), size as dpdk_sys::size_t, align as u32) as *mut u8;
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let align = layout.align();

        return rte_realloc(
            ptr as *mut libc::c_void,
            new_size as dpdk_sys::size_t,
            align as u32,
        ) as *mut u8;
    }
}

unsafe impl Allocator for DPDKAllocator {
    fn allocate(&self, layout: Layout) -> Result<std::ptr::NonNull<[u8]>, std::alloc::AllocError> {
        let ptr = unsafe { self.alloc(layout) };
        let slice = unsafe { slice::from_raw_parts_mut(ptr, layout.size()) };
        std::ptr::NonNull::new(slice)
            .ok_or(AllocError {})
    }

    unsafe fn deallocate(&self, ptr: std::ptr::NonNull<u8>, layout: Layout) {
        unsafe { self.dealloc(ptr.as_ptr(), layout)}
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<std::ptr::NonNull<[u8]>, AllocError> {
        let ptr = unsafe { self.alloc_zeroed(layout) };
        let slice = unsafe { slice::from_raw_parts_mut(ptr, layout.size()) };
        std::ptr::NonNull::new(slice)
            .ok_or(AllocError {})
    }

    unsafe fn grow(
        &self,
        ptr: std::ptr::NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<std::ptr::NonNull<[u8]>, AllocError> {
        debug_assert!(
            new_layout.size() >= old_layout.size(),
            "`new_layout.size()` must be greater than or equal to `old_layout.size()`"
        );

        let ptr = unsafe { self.realloc(ptr.as_ptr(), old_layout, new_layout.size()) };
        let slice = unsafe { slice::from_raw_parts_mut(ptr, new_layout.size()) };
        std::ptr::NonNull::new(slice)
            .ok_or(AllocError {})
    }    
}

// #[global_allocator]
pub const DPDK_ALLOCATOR: DPDKAllocator = DPDKAllocator {};

#[repr(transparent)]
pub struct DPDKBox<T: ?Sized>(Box<T, DPDKAllocator>);

impl<T: Sized> DPDKBox<T> {
    pub fn new(val: T) -> Self {
        Self(Box::new_in(val, DPDK_ALLOCATOR))
    }

    /// 
    /// Creates a new instance with uninitalized backing memory. 
    ///
    /// # Safety
    ///
    /// Returns uninitalized memory. 
    /// 
    /// # Panics
    /// 
    /// Panics on failure to allocate
    pub unsafe fn new_uninit() -> Self {
        Self(Box::new_uninit_in(DPDK_ALLOCATOR).assume_init())
    }

    /// 
    /// Creates a new instance with zeroed backing memory. 
    ///
    /// # Safety
    ///
    /// Returns uninitalized memory. 
    /// 
    /// # Panics
    /// 
    /// Panics on failure to allocate
    pub unsafe fn new_zeroed() -> Self {
        Self(Box::new_zeroed_in(DPDK_ALLOCATOR).assume_init())
    }
}