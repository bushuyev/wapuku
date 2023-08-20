use std::alloc::{GlobalAlloc, Layout};

extern "C" {

    pub fn on_alloc(size: usize, align: usize, pointer: *mut u8);


    pub fn on_dealloc(size: usize, align: usize, pointer: *mut u8);


    pub fn on_alloc_zeroed(size: usize, align: usize, pointer: *mut u8);

    pub fn on_realloc(
        old_pointer: *mut u8,
        new_pointer: *mut u8,
        old_size: usize,
        new_size: usize,
        align: usize,
    );
}



#[derive(Debug)]
pub struct TracingAllocator<A>(pub A)
    where
        A: GlobalAlloc;

unsafe impl<A> GlobalAlloc for TracingAllocator<A>
    where
        A: GlobalAlloc,
{
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();
        let pointer = self.0.alloc(layout);
        on_alloc(size, align, pointer);
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        let size = layout.size();
        let align = layout.align();
        self.0.dealloc(pointer, layout);
        on_dealloc(size, align, pointer);
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();
        let pointer = self.0.alloc_zeroed(layout);
        on_alloc_zeroed(size, align, pointer);
        pointer
    }

    unsafe fn realloc(&self, old_pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old_size = layout.size();
        let align = layout.align();
        let new_pointer = self.0.realloc(old_pointer, layout, new_size);
        on_realloc(old_pointer, new_pointer, old_size, new_size, align);
        new_pointer
    }
}