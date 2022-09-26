//! Allocate memory on early time before the [`mm::early_init`] call. The implementation simply
//! returns the heap base address and set the heap base by adding the allocation size.
//!
//! **Note**: [`mm::set_heap_base_addr`] must be called before using this mod. After calling
//! [`mm::early_init`], any function of this mod should not be called.
//!
//! [`mm::early_init`]: crate::mm::early_init
//! [`mm::set_heap_base_addr`]: crate::mm::set_heap_base_addr

use core::mem::size_of;
use crate::util::align::align_up_of;
use super::HEAP_BASE;


/// Allocate `count` object of `T`. This will alloc `count * size_of::<T>()` bytes memory.
///
/// **Note**: The return address is default aligned with `T`.
pub fn alloc_obj<T>(count: usize) -> *mut T {
    let heap_base = unsafe { HEAP_BASE };
    let base = align_up_of::<T>(heap_base);
    unsafe {
        HEAP_BASE = base + size_of::<T>() * count;
    }

    base as _
}
