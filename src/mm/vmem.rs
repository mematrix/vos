//! Non-contiguous memory allocation. This mod provides a mechanism via [`vmalloc`] where
//! non-contiguous physically memory can be used that is contiguous in virtual memory.
//!
//! [`vmalloc`]: self::vmalloc

pub fn vmalloc(size: usize) -> *mut u8 {
    0usize as _
}

pub fn vfree(ptr: *mut u8) {
    //
}
