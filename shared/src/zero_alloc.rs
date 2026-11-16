//! A global allocator that wipes every block when it is freed.
//!
//! The container password passes through many buffers that no code of ours owns (the decrypted answer, the JSON text of the
//! request, the buffers of the HTTP library). `Zeroize` can only wipe what we hold; this allocator wipes **all** heap
//! memory of the process when it is released, so a freed copy of the password does not stay behind in a long-running
//! process (nanokvm keeps running after it typed the password). It does not cover the stack, CPU registers, swap, or a
//! block that is still alive.
use std::alloc::{GlobalAlloc, Layout, System};
use zeroize::Zeroize;

/// Use it as `#[global_allocator] static ALLOC: vermouth_common::ZeroOnFree = vermouth_common::ZeroOnFree;`
pub struct ZeroOnFree;

unsafe fn wipe(ptr: *mut u8, len: usize) {
    if !ptr.is_null() && len > 0 {
        std::slice::from_raw_parts_mut(ptr, len).zeroize();
    }
}

unsafe impl GlobalAlloc for ZeroOnFree {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        System.alloc(layout)
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        System.alloc_zeroed(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        wipe(ptr, layout.size());
        System.dealloc(ptr, layout)
    }

    // Never resize in place or let the system copy: the old block must be wiped after the data moved.
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_layout = Layout::from_size_align_unchecked(new_size, layout.align());
        let new_ptr = System.alloc(new_layout);
        if !new_ptr.is_null() {
            std::ptr::copy_nonoverlapping(ptr, new_ptr, layout.size().min(new_size));
            self.dealloc(ptr, layout);
        }
        new_ptr
    }
}
