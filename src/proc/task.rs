//! Task struct definitions. `task` is the basic scheduler unit on the CPU hart (On user mode, a `task`
//! is also known as `thread`).

use core::ptr::addr_of_mut;
use crate::proc::kernel_stack::KernelStack;
use crate::smp::HartFrameInfo;
use crate::util::list::List;


/// Status of a task.
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum TaskStatus {
    /// Task is ready to run.
    Ready = 0,
    /// Task is currently running on the CPU.
    Running = 1,
    /// Interruptible sleeping status. Can be wakeup by signal.
    InterruptibleSleep = 2,
    /// Uninterruptible sleeping status. Any async signal are ignored.
    UninterruptibleSleep = 3,
    /// Task dead, destroy all resource except for the `TaskInfo` struct.
    DeadZombie = 4,
    /// Task dead, destroy all resource including the `TaskInfo` struct.
    Dead = 5,
}

impl TaskStatus {
    pub const fn val(self) -> u32 {
        self as u32
    }
}

/// Task struct.
#[repr(C)]
pub struct TaskInfo {
    frame: TaskTrapFrame,
    pub(crate) list: List,
    tid: u32,
    status: TaskStatus,
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

    /// Get the task status.
    #[inline(always)]
    pub fn status(&self) -> TaskStatus {
        self.status
    }

    /// Set task status.
    #[inline(always)]
    pub fn set_status(&mut self, status: TaskStatus) {
        self.status = status;
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
    pub fn trap_frame(&mut self) -> &mut TaskTrapFrame {
        &mut self.frame
    }

    /// Get the trap frame object ptr.
    #[inline(always)]
    pub fn get_trap_frame_ptr(&mut self) -> *mut TaskTrapFrame {
        addr_of_mut!(self.frame)
    }

    /// Convert a `TrapFrame` ptr to the `TaskInfo` ptr.
    #[inline]
    pub unsafe fn from_trap_frame_ptr(frame: *mut TaskTrapFrame) -> *mut TaskInfo {
        container_of_mut!(frame, TaskInfo, frame)
    }
}


/// The task trap frame is set into a structure and packed into each hart's `sscratch` register.
/// This allows for quick reference and full context switch handling. To make offsets easier,
/// everything will be a usize (8 bytes).
///
/// If the task is a thread of user process, then `kernel_stack` points to the [`KernelStack`]
/// object start address. If the task is a kernel thread, then `kernel_stack` points to the
/// stack memory used by the kernel thread (currently the stack size is 4 pages which is 16KiB).
///
/// [`KernelStack`]: crate::proc::kernel::KernelStack
#[repr(C)]
#[derive(Copy, Clone)]
pub struct TaskTrapFrame {
    // 0 - 255
    pub regs: [usize; 32],
    // 256 - 511
    pub fregs: [usize; 32],
    // 512
    pub pc: usize,
    // 520
    pub cpu_stack: *const HartFrameInfo,
    // 528
    pub kernel_stack: *mut KernelStack,
    // 536
    pub satp: usize,
    // 544
    pub qm: usize,
    // 552
    pub pid: usize,
    // 560
    pub mode: usize,
}
