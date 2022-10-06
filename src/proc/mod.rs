//! Controls the lifetime of a process, provides the interfaces to operate with a process.

pub(crate) mod task;
mod idle;
mod kernel_stack;
mod kernel_thread;

/// Kernel stack and kernel thread structs and functions definition.
pub(crate) mod kernel {
    // Re-export on `kernel` mod.
    pub use super::kernel_thread::*;
    pub use super::kernel_stack::*;
    pub use super::idle::create_idle_thread;
}


pub fn init() {
    //
}
