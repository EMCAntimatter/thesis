use dpdk_sys::{rte_free, rte_malloc, rte_realloc, rte_zmalloc};

use serde::{Deserialize, Serialize};

use core::slice;
use std::{
    alloc::{AllocError, Allocator, GlobalAlloc, Layout},
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct DPDKAllocator {}

unsafe impl GlobalAlloc for DPDKAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();
        rte_malloc(std::ptr::null(), size as dpdk_sys::size_t, align as u32) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        rte_free(ptr as *mut libc::c_void);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        rte_zmalloc(std::ptr::null(), size as dpdk_sys::size_t, align as u32) as *mut u8
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let align = layout.align();

        rte_realloc(
            ptr as *mut libc::c_void,
            new_size as dpdk_sys::size_t,
            align as u32,
        ) as *mut u8
    }
}

unsafe impl Allocator for DPDKAllocator {
    fn allocate(&self, layout: Layout) -> Result<std::ptr::NonNull<[u8]>, std::alloc::AllocError> {
        let ptr = unsafe { self.alloc(layout) };
        let slice = unsafe { slice::from_raw_parts_mut(ptr, layout.size()) };
        std::ptr::NonNull::new(slice).ok_or(AllocError {})
    }

    unsafe fn deallocate(&self, ptr: std::ptr::NonNull<u8>, layout: Layout) {
        unsafe { self.dealloc(ptr.as_ptr(), layout) }
    }

    fn allocate_zeroed(&self, layout: Layout) -> Result<std::ptr::NonNull<[u8]>, AllocError> {
        let ptr = unsafe { self.alloc_zeroed(layout) };
        let slice = unsafe { slice::from_raw_parts_mut(ptr, layout.size()) };
        std::ptr::NonNull::new(slice).ok_or(AllocError {})
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
        std::ptr::NonNull::new(slice).ok_or(AllocError {})
    }
}

impl DPDKAllocator {
    /// Returns the allocate single uninit of this [`DPDKAllocator`].
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn allocate_single_uninit<T: Sized>(
        &self,
    ) -> Result<NonNull<T>, std::alloc::AllocError> {
        let layout = Layout::new::<T>();
        let ptr: *mut T = unsafe { self.alloc(layout) }.cast();
        NonNull::new(ptr).ok_or(AllocError {})
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn allocate_array_uninit<T: Sized>(
        &self,
        size: usize,
    ) -> Result<NonNull<[T]>, std::alloc::AllocError> {
        let layout = Layout::array::<T>(size).unwrap();
        let ptr: *mut T = unsafe { self.alloc(layout) }.cast();
        let slice = unsafe { slice::from_raw_parts_mut(ptr, size) };
        NonNull::new(slice).ok_or(AllocError {})
    }

    /// Returns the allocate array fixed uninit of this [`DPDKAllocator`].
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn allocate_array_fixed_uninit<const SIZE: usize, T: Sized>(
        &self,
    ) -> Result<NonNull<[T; SIZE]>, std::alloc::AllocError> {
        let layout = Layout::new::<[T; SIZE]>();
        let ptr: *mut [T; SIZE] = unsafe { self.alloc(layout) }.cast();
        NonNull::new(ptr).ok_or(AllocError {})
    }

    /// Returns the allocate single zeroed of this [`DPDKAllocator`].
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn allocate_single_zeroed<T: Sized>(
        &self,
    ) -> Result<NonNull<T>, std::alloc::AllocError> {
        let layout = Layout::new::<T>();
        let ptr: *mut T = unsafe { self.alloc_zeroed(layout) }.cast();
        NonNull::new(ptr).ok_or(AllocError {})
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn allocate_array_zeroed<T: Sized>(
        &self,
        size: usize,
    ) -> Result<NonNull<[T]>, std::alloc::AllocError> {
        let layout = Layout::array::<T>(size).unwrap();
        let ptr: *mut T = unsafe { self.alloc_zeroed(layout) }.cast();
        let slice = unsafe { slice::from_raw_parts_mut(ptr, size) };
        NonNull::new(slice).ok_or(AllocError {})
    }

    /// Returns the allocate array fixed zeroed of this [`DPDKAllocator`].
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    ///
    /// # Safety
    ///
    /// .
    pub unsafe fn allocate_array_fixed_zeroed<const SIZE: usize, T: Sized>(
        &self,
    ) -> Result<NonNull<[T; SIZE]>, std::alloc::AllocError> {
        let layout = Layout::new::<[T; SIZE]>();
        let ptr: *mut [T; SIZE] = unsafe { self.alloc_zeroed(layout) }.cast();
        NonNull::new(ptr).ok_or(AllocError {})
    }
}

// #[global_allocator]
pub const DPDK_ALLOCATOR: DPDKAllocator = DPDKAllocator {};

impl Default for DPDKAllocator {
    fn default() -> Self {
        DPDK_ALLOCATOR
    }
}

#[repr(transparent)]
pub struct DPDKBox<T: ?Sized>(Box<T, DPDKAllocator>);

impl<T: ?Sized> AsMut<T> for DPDKBox<T> {
    fn as_mut(&mut self) -> &mut T {
        self.0.as_mut()
    }
}

impl<T: ?Sized> AsRef<T> for DPDKBox<T> {
    fn as_ref(&self) -> &T {
        self.0.as_ref()
    }
}

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

impl<T: ?Sized> DPDKBox<T> {
    pub fn leak<'a>(self) -> &'a mut T {
        Box::leak(self.0)
    }

    pub fn from_non_null(val: NonNull<T>) -> Self {
        Self(unsafe { Box::from_raw_in(val.as_ptr(), DPDK_ALLOCATOR) })
    }
}

impl<T: ?Sized> Deref for DPDKBox<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl<T: ?Sized> DerefMut for DPDKBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}

#[cfg(test)]
mod test {
    use std::alloc::{GlobalAlloc, Layout};

    use ::test::Bencher;

    use crate::config::DPDKConfig;

    use super::DPDK_ALLOCATOR;
    const SLAB_SIZE: usize = 1600;

    #[bench]
    fn bench_allocator_thrashing(b: &mut Bencher) {
        DPDKConfig {
            cores: crate::config::CoreConfig::List(vec![1, 2, 3]),
            main_lcore: None,
            service_core_mask: Some(0b11),
            pci_options: crate::config::PCIOptions::PCI {
                blocked_devices: vec![],
                allowed_devices: vec![],
            },
            virtual_devices: vec![],
            ..Default::default()
        }
        .apply()
        .expect("Error configuring EAL");
        b.iter(|| unsafe {
            (0..(10_000_000 / SLAB_SIZE))
                .map(|_| DPDK_ALLOCATOR.allocate_array_zeroed::<u128>(SLAB_SIZE))
                .for_each(|c| {
                    let c = c.unwrap();
                    let ptr = c.as_ptr() as *mut _;
                    DPDK_ALLOCATOR.dealloc(ptr, Layout::for_value(&c))
                })
        })
    }
}
