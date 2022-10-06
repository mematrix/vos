//! Idle kernel thread. Whenever a hart is free, this idle thread will be scheduled.

use core::mem::zeroed;
use crate::arch::cpu;
use crate::proc::task::TaskInfo;
use crate::smp::{CpuInfo, current_cpu_frame};


pub fn create_idle_thread() -> TaskInfo {
    // todo: alloc `TaskInfo` from kmem.
    let mut task: TaskInfo = unsafe { zeroed() };
    let frame = task.trap_frame_mut();
    frame.cpu_stack = current_cpu_frame();

    task
}

extern "C"
fn idle_work(data: *mut ()) -> usize {
    let cur_cpu = unsafe {
        let self_task = data as *const TaskInfo;
        let cpu_stack = (*self_task).trap_frame().cpu_stack;
        let info = (*cpu_stack).tp as *const CpuInfo;
        &*info
    };

    let mut time = cpu::read_time();
    info!("[Idle] Task begin at cpu time: {}", time);
    let interval_1s = cur_cpu.get_timebase_freq();
    let interval_4s = interval_1s << 2;
    loop {
        // Idle task print every 4s.
        let cur = cpu::read_time();
        if cur >= time + interval_4s {
            info!("[Idle] Current cpu time: {}", cur);
            time = cur;
        }
    }
}
