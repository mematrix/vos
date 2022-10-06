//! Task struct definitions. `task` is the basic scheduler unit on the CPU hart (On user mode, a `task`
//! is also known as `thread`).

use core::ptr::addr_of_mut;
use crate::proc::kernel::KernelTrapFrame;
use crate::smp::HartFrameInfo;
use crate::util::list::List;


/// Status of a task.
#[repr(u8)]
#[derive(Copy, Clone)]
#[derive(Eq, PartialEq)]
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
    #[inline(always)]
    pub const fn val(self) -> u32 {
        self as u8 as u32
    }
}

/// Task type.
#[repr(u8)]
#[derive(Copy, Clone)]
#[derive(Eq, PartialEq)]
pub enum TaskType {
    /// Kernel mode thread.
    Kernel = 0,
    /// User mode thread.
    User = 1,
}

impl TaskType {
    #[inline(always)]
    pub const fn val(self) -> u8 {
        self as u8
    }
}


/// Task struct.
#[repr(C)]
pub struct TaskInfo {
    frame: TaskTrapFrame,
    pub(crate) list: List,
    tid: u32,
    status: TaskStatus,
    /// Bits flag \[7:0].
    /// * bit 0: TaskType.
    /// * bit 7: If a user thread is running in kernel mode.
    ty_flag: u8,
    /// Task schedule priority. In most time this is equal to the `priority`.
    sched_priority: i8,
    /// Task static priority. The higher the value, the higher the priority. **Realtime task**
    /// has a priority that between in \[51, 60] (10 levels). **Normal task** has a priority
    /// of \[-10, 10] (21 levels), `0` means the most normal priority.
    priority: i8,
    /// Thread exit code.
    exit_code: usize
    // todo: Process info
}

const TY_MASK_USER_TRAP_IN: u8 = 0b1000_0000u8;

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

    /// Get the task type.
    #[inline(always)]
    pub fn task_type(&self) -> TaskType {
        if (self.ty_flag & 0b0001u8) == 0 {
            TaskType::Kernel
        } else {
            TaskType::User
        }
    }

    /// Set the task type. Only can be called by the constructor.
    #[inline(always)]
    pub(super) fn set_task_type(&mut self, ty: TaskType) {
        self.ty_flag = (self.ty_flag & !0b0000_0001u8) | ty.val();
    }

    /// Check if a user thread is running in the kernel mode.
    #[inline(always)]
    pub fn is_user_in_kernel_mode(&self) -> bool {
        (self.ty_flag & TY_MASK_USER_TRAP_IN) != 0
    }

    /// Notify that the user thread enter the kernel mode.
    #[inline(always)]
    pub fn user_enter_kernel(&mut self) {
        self.ty_flag |= TY_MASK_USER_TRAP_IN;
    }

    /// Notify that the user thread exit the kernel mode.
    #[inline(always)]
    pub fn user_exit_kernel(&mut self) {
        self.ty_flag &= !TY_MASK_USER_TRAP_IN;
    }

    /// Get the task priority.
    #[inline(always)]
    pub fn priority(&self) -> i8 {
        self.priority
    }

    /// Set the task priority. Caller must guard the priority value is in the valid range.
    #[inline(always)]
    pub fn set_priority(&mut self, priority: i8) {
        self.priority = priority;
    }

    /// Check if the task is a realtime task.
    #[inline(always)]
    pub fn is_realtime_task(&self) -> bool {
        self.priority > 50i8
    }

    /// Get the task schedule priority.
    #[inline(always)]
    pub fn sched_priority(&self) -> i8 {
        self.sched_priority
    }

    /// Set the task schedule priority. Caller must guard the priority value is in the valid range.
    #[inline(always)]
    pub fn set_sched_priority(&mut self, sched_priority: i8) {
        self.sched_priority = sched_priority;
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
/// If the task is a thread of user process, then `kernel_stack` points to the [`KernelTrapFrame`]
/// object which is part of the full **Kernel Stack**. If the task is a kernel thread, then
/// `kernel_stack` points to the stack memory used by the kernel thread (currently the stack size
/// is 4 pages which is 16KiB).
///
/// [`KernelTrapFrame`]: crate::proc::kernel::KernelTrapFrame
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
    pub kernel_stack: *mut KernelTrapFrame,
    // 536
    pub satp: usize,
    // 544
    pub qm: usize,
    // 552
    pub pid: usize,
    // 560
    pub mode: usize,
}
