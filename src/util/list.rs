//! Circular doubly linked list implementation.

use core::ptr::null_mut;


/// Double linked list. Embedded in the actual entry struct to give the entry struct
/// the linked list capability.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct List {
    pub prev: *mut List,
    pub next: *mut List,
}

impl List {
    pub const fn new() -> Self {
        Self {
            prev: null_mut(),
            next: null_mut(),
        }
    }
}
