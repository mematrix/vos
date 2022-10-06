//! SMP config. Provides a simple struct to access the per-cpu cache data.

mod cpu_info;
mod per_cpu;
mod cpu_stack;

pub use cpu_info::CpuInfo;
pub use cpu_stack::*;
pub use per_cpu::PerCpuPtr;


/// SMP CPU count.
static mut CPU_COUNT: usize = 0;

/// Init the smp info on boot time. Alloc and init the **per-cpu** stack frame data.
pub fn boot_init(cpu_count: usize) {
    unsafe {
        debug_assert!(CPU_COUNT == 0);
        CPU_COUNT = cpu_count;
    }

    init_per_cpu_stack(cpu_count);
}


/// Get cpu count.
pub fn get_cpu_count() -> usize {
    unsafe {
        CPU_COUNT
    }
}
