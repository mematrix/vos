//! Provides the necessary structs used by the **Rust `alloc` crate**.
//!
//! The global allocator allows us to use the data structures in the core library, such
//! as a linked list or B-tree.

use core::alloc::{GlobalAlloc, Layout};
use crate::mm::{kfree, kzalloc};


// The global allocator is a static constant to a global allocator
// structure. We don't need any members because we're using this
// structure just to implement alloc and dealloc.
struct OsGlobalAlloc;

unsafe impl GlobalAlloc for OsGlobalAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // We align to the next page size so that when
        // we divide by PAGE_SIZE, we get exactly the number
        // of pages necessary.
        kzalloc(layout.size(), 0)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        // We ignore layout since our allocator uses ptr_start -> last
        // to determine the span of an allocation.
        kfree(ptr);
    }
}

#[global_allocator]
static GA: OsGlobalAlloc = OsGlobalAlloc {};

// If for some reason alloc() in the global allocator gets null_mut(), then we come here.
// This is a divergent function, so we call panic to let the tester know what's going on.
// #[alloc_error_handler]
// pub fn alloc_error(l: Layout) -> ! {
//     panic!(
//         "Allocator failed to allocate {} bytes with {}-byte alignment.",
//         l.size(),
//         l.align()
//     );
// }
// See unstable feature 'default_alloc_error_handler'.
