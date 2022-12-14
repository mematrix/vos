//! Controls the lifetime of a process, provides the interfaces to operate with a process.

pub(crate) mod task;
mod idle;
mod kernel_context;
mod kernel_stack;
mod kernel_thread;
mod kernel_test;

/// Kernel stack and kernel thread structs and functions definition. This mod should only be
/// used on the kernel thread task or a task trapped in kernel mode.
pub(crate) mod kernel {
    // Re-export on `kernel` mod.
    pub use super::kernel_thread::*;
    pub use super::kernel_stack::*;
    pub use super::idle::build_idle_thread;

    // Re-export on `kernel::ctx` mod.
    pub mod ctx {
        pub use crate::proc::kernel_context::*;
    }
}

pub use kernel_test::add_test_kernel_threads;


pub fn init() {
    //
}
