//! Context info access helper functions. Should only be used in a kernel thread. It is an
//! **Undefined Behavior** if this mod is called on a non-kernel thread.

use crate::arch::cpu;
use crate::proc::task::TaskInfo;
use crate::smp::CpuInfo;


/// Get task info struct of self.
#[inline]
pub fn self_task_info<'a>() -> &'a TaskInfo {
    unsafe { &*(cpu::sscratch_read() as *const TaskInfo) }
}

/// Get the CPU info that current task is running on.
///
/// **Note**: This function **must** be used within the **preempt-disabled** context.
#[inline]
pub unsafe fn this_cpu_info<'a>() -> &'a CpuInfo {
    let cpu = self_task_info().trap_frame().cpu_stack;
    unsafe {
        &*((*cpu).tp as *const CpuInfo)
    }
}
