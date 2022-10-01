//! Task struct definitions. `task` is the basic scheduler unit on the CPU hart (On user mode, a `task`
//! is also known as `thread`).

use crate::sc::TrapFrame;

pub struct TaskInfo {
    frame: TrapFrame,
    tid: u32,
    // Process info
}

impl TaskInfo {
    /// Get the `tid`.
    #[inline(always)]
    pub fn get_tid(&self) -> u32 {
        self.tid
    }

    /// Set the task `tid`.
    #[inline(always)]
    pub fn set_tid(&mut self, tid: u32) {
        self.tid = tid;
    }

    /// Get the trap frame object ref.
    #[inline(always)]
    pub fn get_trap_frame(&mut self) -> &mut TrapFrame {
        &mut self.frame
    }

    /// Convert a `TrapFrame` ptr to the `TaskInfo` ptr.
    #[inline]
    pub unsafe fn from_trap_frame_ptr(frame: *mut TrapFrame) -> *mut TaskInfo {
        container_of_mut!(frame, TaskInfo, frame)
    }
}
