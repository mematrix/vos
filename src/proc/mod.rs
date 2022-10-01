//! Controls the lifetime of a process, provides the interfaces to operate with a process.

pub(crate) mod task;
mod idle;

use crate::proc::task::TaskInfo;


pub fn init() {
    //
}

pub fn create_idle_kernel_thread() -> TaskInfo {
    idle::build_idle_task_info()
}
