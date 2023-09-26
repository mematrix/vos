//! Provides the **compressed** pointer type to manage pointers get from [`mm::page`] mod.
//!
//! [`mm::page`]: crate::mm::page

struct CompressedPtr<const ORDER: usize> {
    union_ptr: usize,
}

impl<const ORDER: usize> CompressedPtr<ORDER> {
    pub const fn new(ptr: usize, data: usize) -> Self {
        Self {
            union_ptr:
        }
    }
}
