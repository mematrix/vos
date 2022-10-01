//! SMP config. Provides a simple struct to access the per-cpu cache data.

mod cpu_info;
pub(crate) mod cpu;

pub use cpu_info::CpuInfo;


/// SMP CPU count.
static mut CPU_COUNT: usize = 0;

/// Init the smp info.
pub fn init(cpu_count: usize) {
    unsafe {
        debug_assert!(CPU_COUNT == 0);
        CPU_COUNT = cpu_count;
    }
}
