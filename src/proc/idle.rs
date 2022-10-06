//! Idle kernel thread. Whenever a hart is free, this idle thread will be scheduled.

use core::ptr::null_mut;
use crate::arch::cpu;
use crate::proc::kernel::{build_kernel_thread_on_place, ctx};
use crate::proc::task::TaskInfo;


pub fn build_idle_thread(task: *mut TaskInfo) {
    unsafe {
        // We dropped the return ptr value, as it is the same as `task` and not used.
        let _ = build_kernel_thread_on_place(idle_work, null_mut(), task).build();
    }
}

extern "C"
fn idle_work(_data: *mut ()) -> usize {
    let cur_cpu = unsafe {
        // SAFETY: Idle task will always run on the same hart, so the current CPU info ptr
        // will never change even after the context switch.
        ctx::this_cpu_info()
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
