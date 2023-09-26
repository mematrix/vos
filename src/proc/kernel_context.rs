//! Context info access helper functions. Should only be used in the kernel mode thread. It is
//! an **Undefined Behavior** if this mod is called on a thread not in the kernel mode.

use crate::arch::cpu;
use crate::proc::task::TaskInfo;
use crate::smp::{CpuInfo, current_cpu_info};


/// Get task info struct of self.
#[inline(always)]
pub fn self_task_info<'a>() -> &'a TaskInfo {
    unsafe { &*(cpu::sscratch_read() as *const TaskInfo) }
}

#[inline(always)]
pub fn self_task_info_mut<'a>() -> &'a mut TaskInfo {
    unsafe { &mut *(cpu::sscratch_read() as *mut TaskInfo) }
}

/// Get the CPU info that current task is running on.
///
/// **Note**: This function **must** be used within the **preempt-disabled** context.
#[inline(always)]
pub unsafe fn this_cpu_info() -> &'static CpuInfo {
    current_cpu_info()
}
