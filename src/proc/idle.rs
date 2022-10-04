//! Idle kernel thread. Whenever a hart is free, this idle thread will be scheduled.

use core::mem::zeroed;
use crate::proc::task::TaskInfo;
use crate::smp::current_cpu_frame;


pub fn build_idle_task_info() -> TaskInfo {
    // todo: alloc `TaskInfo` from kmem.
    let mut task: TaskInfo = unsafe { zeroed() };
    let frame = task.trap_frame();
    frame.cpu_stack = current_cpu_frame();

    task
}
