//! Task struct definitions. `task` is the basic scheduler unit on the CPU hart (On user mode, a `task`
//! is also known as `thread`).

use core::ptr::addr_of_mut;
use crate::sc::TrapFrame;


#[repr(C)]
pub struct TaskInfo {
    frame: TrapFrame,
    tid: u32,
    // Process info
    /// Thread exit code.
    exit_code: usize
}

impl TaskInfo {
    /// Get the `tid`.
    #[inline(always)]
    pub fn tid(&self) -> u32 {
        self.tid
    }

    /// Set the task `tid`.
    #[inline(always)]
    pub fn set_tid(&mut self, tid: u32) {
        self.tid = tid;
    }

    /// Get thread exit code.
    #[inline(always)]
    pub fn exit_code(&self) -> usize {
        self.exit_code
    }

    /// Set thread exit code.
    #[inline(always)]
    pub fn set_exit_code(&mut self, exit_code: usize) {
        self.exit_code = exit_code;
    }

    /// Get the trap frame object ref.
    #[inline(always)]
    pub fn trap_frame(&mut self) -> &mut TrapFrame {
        &mut self.frame
    }

    /// Get the trap frame object ptr.
    #[inline(always)]
    pub fn get_trap_frame_ptr(&mut self) -> *mut TrapFrame {
        addr_of_mut!(self.frame)
    }

    /// Convert a `TrapFrame` ptr to the `TaskInfo` ptr.
    #[inline]
    pub unsafe fn from_trap_frame_ptr(frame: *mut TrapFrame) -> *mut TaskInfo {
        container_of_mut!(frame, TaskInfo, frame)
    }
}
