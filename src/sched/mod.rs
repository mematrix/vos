//! Scheduler service module.
//!
//! Some registers need to be saved between the context stitch so that we can resume the
//! process. We packed these registers into a struct [`TaskTrapFrame`] with some additional
//! information. The additional info like `cpu_stack` and `kernel_stack` are used as quick
//! reference and to simplify the context switch code; and other additional info can be used
//! in the trap handler.
//!
//! [`TaskTrapFrame`] is binding to the **Thread**, each time a **Thread** is scheduled, the
//! associated [`TaskTrapFrame`]'s physical address will be saved in the `sscratch` register.
//!
//! `cpu_stack` (defined in [`HartTrapStack`]) is binding to the current **hart** that execute
//! the instructions, each time a **Thread** is scheduled to current **hart**, the `cpu_stack`
//! field will update to reference the [`HartTrapStack`] object of current **hart**. To
//! simplify the asm code, the `cpu_stack` will point to the [`HartFrameInfo`] object which
//! is the inner object of the [`HartTrapStack`].
//!
//! # API Usage
//!
//! **Note**: All scheduler APIs **must** be called on a **preempt-disabled** context.
//!
//! # Context Registers:
//!
//! | Registers | Description |
//! | :-------: | ----------- |
//! | `x0`~`x31` | Generic registers. Note that `x0` is read-only constant zero. |
//! | `f0`~`f31` | Generic floating registers. See [Context Status] |
//! | `pc` | The instruction counter register. |
//!
//! # Context Status
//!
//! RISC-V has several extension **context status** field in the `sstatus` register: `FS[1:0]`,
//! `VS[1:0]`, and `XS[1:0]`. These fields are used to reduce the cost of context save and
//! restore by setting and tracking the current state of the floating-point unit and any other
//! user-mode extensions respectively.
//!
//! The `FS`, `VS`, and `XS` fields use the same status encoding as shown in following table,
//! with the four possible status values being `Off`, `Initial`, `Clean`, and `Dirty`.
//!
//! | Status | `FS` and `VS` Meaning | `XS` Meaning |
//! | :----: | --------------------- | ------------ |
//! | 0 | Off | All off |
//! | 1 | Initial | None dirty or clean, some on |
//! | 2 | Clean | None dirty, some clean |
//! | 3 | Dirty | Some dirty |
//!
//! See the RISC-V Privileged Spec Chapter 3.1.6.6 to get more detailed information.
//!
//! During a context save, we need only write out the corresponding state if its status is
//! `Dirty`, and can then reset the extension's status to `Clean`.
//!
//! > A context save will happen on the time of a trap occurs or a thread gives up the CPU time
//! slice with the schedule API call. If a user process thread gives up the CPU time slice, a
//! trap will occur due to the sys-call; but if a kernel thread gives up the CPU time slice, no
//! traps occur so we need do context save on the schedule API.
//!
//! During a context restore, the context need only be loaded from memory if the status is
//! `Clean` (it should never be `Dirty` at restore). If the status is `Initial`, the context
//! must be set to an initial constant value on context restore to avoid a security hole, but
//! this can be done without accessing memory.
//!
//! > A context restore will happen on the time of a trap handler returns or a thread is selected
//! to be scheduled. **Context restore on a trap handler** has a little difference: we only need
//! restore the context if the status is `Dirty` (not `Clean`) which means the trap handler
//! modifies the corresponding state so we need to restore it; If a context switching occurs (for
//! example in the timer interrupt), the trap handler context restore code will never be executed.
//! >
//! > Currently we do not set an extension's status to `Initial` except for the boot time, so we
//! do not handle the `Initial` status on a context restore.
//!
//! ## Floating registers status
//!
//! The `FS` field encodes the status of the floating-point unit state, including the
//! floating-point registers `f0`–`f31` and the CSRs `fcsr`, `frm`, and `fflags`.
//!
//! The `FS` field is set to `Initial` on boot to enable the floating-point instructions.
//!
//! [Context Status]: #context-status
//! [`TaskTrapFrame`]: crate::proc::task::TaskTrapFrame
//! [`HartTrapStack`]: crate::smp::HartTrapStack
//! [`HartFrameInfo`]: crate::smp::HartFrameInfo

mod trap;
mod scheduler;
mod preempt;

// Re-export all.
pub use scheduler::*;

use crate::arch::cpu;
use crate::proc::task::{TaskStatus, TaskType};
use crate::smp::{current_cpu_frame, current_cpu_info};


/// Init scheduler service.
///
/// 1. Setup the idle thread.
/// 2. Set `sstatus->sPIE` to 1 so that interrupt is enabled after the `sret` instruction in
/// the `switch_to_task` function.
pub(crate) fn init() {
    // Init scheduler, set the idle task.
    init_and_set_idle_task();

    // Set sPIE flag.
    cpu::sstatus_set_spie();
}

/// Schedule a task on current CPU.
///
/// 1. Select a task of user process thread or kernel thread.
/// 2. Set the `sstatus->sPP` to correspond the select task type.
/// 3. Set timer event to next context switching time.
/// 4. Call `switch_to_task` to restore context and switch to the selected task.
pub(crate) fn schedule() /* -> ! */ {
    let task = find_ready_task_or_idle();
    let task_ref = unsafe { &mut *task };

    if task_ref.task_type() == TaskType::Kernel || task_ref.is_user_in_kernel_mode() {
        cpu::sstatus_set_bits(cpu::SSTATUS_SPP_BIT);
    } else {
        cpu::sstatus_clear_bits(cpu::SSTATUS_SPP_BIT);
    }

    let cpu_info = current_cpu_info();
    cpu::stimecmp_write_delta(if task_ref.is_realtime_task() {
        cpu_info.get_time_slice_realtime()
    } else {
        cpu_info.get_time_slice_normal()
    });

    // Get the `TaskTrapFrame`.
    let cpu_frame = current_cpu_frame();
    task_ref.trap_frame_mut().cpu_stack = cpu_frame;
    let trap_frame = if task_ref.task_type() == TaskType::Kernel {
        task_ref.get_trap_frame_ptr() as usize
    } else {
        // Set cpu_stack on both user trap frame and kernel trap frame.
        let kernel_stack = task_ref.trap_frame_mut().kernel_stack;
        unsafe {
            (*kernel_stack).cpu_stack = cpu_frame;
        }

        // User thread trap in kernel mode, restore the kernel stack context.
        if task_ref.is_user_in_kernel_mode() {
            task_ref.trap_frame().kernel_stack as usize
        } else {
            task_ref.get_trap_frame_ptr() as usize
        }
    };
    task_ref.set_status(TaskStatus::Running);
    unsafe {
        switch_to_task(trap_frame);
    }
}

extern "C" {
    fn switch_to_task(trap_frame: usize) -> !;
}

/// Do preempt schedule on the current CPU.
pub(crate) fn preempt_schedule() /* -> ! */ {
    //
}
