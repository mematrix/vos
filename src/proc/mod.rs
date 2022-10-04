//! Controls the lifetime of a process, provides the interfaces to operate with a process.

pub(crate) mod task;
mod kernel_thread;
mod idle;

pub(crate) mod kernel {
    // Re-export on `kernel` mod.
    pub use super::kernel_thread::*;
}

use crate::proc::task::TaskInfo;


pub fn init() {
    //
}

pub fn create_idle_kernel_thread() -> TaskInfo {
    idle::build_idle_task_info()
}
