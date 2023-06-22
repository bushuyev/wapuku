use std::alloc::{GlobalAlloc, Layout};
use log::debug;
use core::{
    cell::UnsafeCell,
    ptr::null_mut,
};
use std::sync::atomic::AtomicUsize;
use std::sync::Mutex;

const PAGE_SIZE: usize = 65536;

pub struct LockedAllocator<T:GlobalAlloc> {
    spin: Mutex<T>,
}

impl<T:GlobalAlloc> LockedAllocator<T> {
    pub const fn new(t: T) -> Self {
        LockedAllocator {
            spin: Mutex::new(t),
        }
    }
}

unsafe impl<T: GlobalAlloc> GlobalAlloc for LockedAllocator<T> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.spin.lock().unwrap().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.spin.lock().unwrap().dealloc(ptr, layout);
    }
}

pub(crate) struct WapukuAllocator {
    used: UnsafeCell<usize>, // bytes
    size: UnsafeCell<usize>, // bytes
}

impl WapukuAllocator {
    pub const fn new() -> Self {
        Self {
            used: UnsafeCell::new(0),
            size: UnsafeCell::new(0),
        }
    }
}

unsafe impl GlobalAlloc for  WapukuAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        debug!("alloc: {:?}", layout);
        let size: &mut usize = &mut *self.size.get();
        let used: &mut usize = &mut *self.used.get();
        // This assumes PAGE_SIZE is always a multiple of the required alignment, which should be true for all practical use.
        // If this is not true, this could go past size.
        let alignment = layout.align();
        let offset = *used % alignment;
        if offset != 0 {
            *used += alignment - offset;
        }

        let requested_size = layout.size();
        let new_total = *used + requested_size;
        if new_total > *size {
            // Request enough new space for this allocation, even if we have some space left over from the last one incase they end up non-contiguous.
            // Round up to a number of pages
            let requested_pages = (requested_size + PAGE_SIZE - 1) / PAGE_SIZE;
            let previous_page_count = core::arch::wasm32::memory_grow(0, requested_pages);
            if previous_page_count == usize::MAX {
                return null_mut();
            }

            let previous_size = previous_page_count * PAGE_SIZE;
            if previous_size != *size {
                // New memory is not contiguous with old: something else allocated in-between.
                // TODO: is handling this case necessary? Maybe make it optional behind a feature?
                // This assumes PAGE_SIZE is always a multiple of the required alignment, which should be true for all practical use.
                *used = previous_size;
                // TODO: in free mode, have minimum alignment used is rounded up to and is maxed with alignment so we can ensure there is either:
                // 1. no space at the end of the page
                // 2. enough space we can add it to the free list
            }
            *size = previous_size + requested_pages * PAGE_SIZE;
        }

        let start = *used;
        *used += requested_size;
        start as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        debug!("dealloc: ptr={:?} {:?}", ptr, layout);
    }

    // unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
    //     debug!("alloc_zeroed: {:?}", layout);
    //     todo!();
    // }
    // 
    // unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
    //     debug!("realloc: ptr={:?} {:?} new_size={}", ptr, layout, new_size);
    //     todo!()
    // }
}