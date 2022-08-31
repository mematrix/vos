//! Page-based memory management.
//!
//! This mod provides some functions to allocate/deallocate the physical memory
//! from the HEAP area. The page size is default set to 4KiB.

// todo: add page alloc fn for discontinuous pages: fn a(s: usize, c: fn(*mut()), u: *mut())

use super::align_val;
use crate::asm::mem_v;


// We will use ALLOC_START to mark the start of the actual
// memory we can dish out.
static mut ALLOC_START: usize = 0;
const PAGE_ORDER: usize = 12;
pub const PAGE_SIZE: usize = 1 << 12;

#[repr(u8)]
enum PageBits {
    Empty = 0,
    Taken = 1 << 0,
    Last = 1 << 1,
}

impl PageBits {
    /// Get the underlying representation value.
    pub const fn val(self) -> u8 {
        self as u8
    }
}


