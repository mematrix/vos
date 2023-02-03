//! Platform-special atomic primitives.

use core::mem::transmute;

mod riscv_atomic_asm;


/// 64-bits CAS wrapper for raw pointer.
#[inline(always)]
pub fn compare_exchange64(ptr: *mut u64, expected: u64, new: u64) -> bool {
    unsafe {
        riscv_atomic_asm::compare_exchange64_lr_sc(ptr, expected, new)
    }
}

/// CAS wrapper for raw pointer points to pointer-sized data.
#[inline(always)]
pub fn compare_exchange_usize(ptr: *mut usize, expected: usize, new: usize) -> bool {
    // when pointer is 64-bits
    unsafe {
        compare_exchange64(ptr as _, transmute(expected), transmute(new))
    }
}
