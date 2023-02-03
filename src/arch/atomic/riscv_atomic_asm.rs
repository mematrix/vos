core::arch::global_asm!(include_str!("riscv_atomic.S"));

extern "C" {
    /// 64-bits CAS implementation by using LR/SC instructions.
    pub fn compare_exchange64_lr_sc(ptr: *mut u64, expected: u64, new: u64) -> bool;
}
