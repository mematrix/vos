//! Scheduler service (or Switching Context) module.
//!
//! Some registers need to be saved between the context stitch so that we can resume the
//! process. We packed these registers into a struct [`TrapFrame`] with some additional
//! information. The additional info like `cpu_stack` and `kernel_stack` are used as quick
//! reference and to simplify the context switch code; and other additional info can be
//! used in the trap handler.
//!
//! [`TrapFrame`] is binding to the **Thread**, each time a **Thread** is scheduled, the
//! associated [`TrapFrame`]'s physical address will be saved in the `sscratch` register.
//!
//! `cpu_stack` (defined in [`TrapStack`]) is binding to the current **hart** that execute
//! the instructions, each time a **Thread** is scheduled to current **hart**, the `cpu_stack`
//! field will update to reference the [`TrapStack`] object of current **hart**.
//! To simplify the asm code, the `cpu_stack` will point to the [`TrapStackFrame`] object
//! which is the inner object of the [`TrapStack`].
//!
//! The context registers:
//!
//! | Registers | Description |
//! | --------- | ----------- |
//! | `x0`~`x31` | Generic registers. Note that `x0` is read-only constant zero. |
//! | `f0`~`f31` | Generic floating registers. More see **`*1*`** |
//! | `pc` | The instruction counter register. |
//!
//! > **`*1*`**: Note: The floating register need to be saved only when the `sstatus->FS`
//! field's value is 3.
//!
//! [`TrapFrame`]: sc::TrapFrame
//! [`TrapStack`]: sc::cpu::TrapStack
//! [`TrapStackFrame`]: sc::cpu::TrapStackFrame

pub(crate) mod cpu;

use core::mem::size_of;
use crate::mm::page::PAGE_SIZE;


/// Alloc and init the **per-cpu** data.
pub fn init(cpu_count: usize) {
    cpu::init_smp(cpu_count);
}


/// The trap frame is set into a structure and packed into each hart's `sscratch` register.
/// This allows for quick reference and full context switch handling.
/// To make offsets easier, everything will be a usize (8 bytes).
///
/// `kernel_stack` points to the [`KernelStack`] object start address.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct TrapFrame {
    // 0 - 255
    pub regs: [usize; 32],
    // 256 - 511
    pub fregs: [usize; 32],
    // 512
    pub pc: usize,
    // 520
    pub cpu_stack: usize,
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

/// Kernel stack context frame. Used when interrupt is enabled while handling the `ecall`
/// sys-call to support single level recursive interrupt.
///
/// This is part definition of the *Full* kernel stack: only contains the high address of
/// the memory, and the low address range is used as the **stack** to run the sys-call func.
///
/// > **Note**: This stack is used only when the interrupt is enabled while we are handling
/// the sys-call. If the interrupt is disabled (This is the default setting when handling
/// a trap), we use the **Trap stack** binding to each hart as the function stack. See
/// [`TrapStack`].
///
/// The *Full* kernel stack is allocated as a single [page], so its size is 4KiB and the
/// available stack range is `[0, 4096 - sizeof::<KernelStack>() - 8]` (8bytes reserved).
/// See [`KernelStack`].
///
/// This definition has the same layout with the head part of [`TrapFrame`], this can simplify
/// the context switch code.
///
/// **Note**: not like the `kernel_stack` field in [`TrapFrame`], the `user_frame` (which is
/// in the same layout position as the `kernel_stack`) field points to the [`TrapFrame`]
/// object's start address.
///
/// [`TrapStack`]: sc::cpu::TrapStack
/// [page]: mm::page
/// [`KernelStack`]: sc::KernelStack
/// [`TrapFrame`]: sc::TrapFrame
#[repr(C)]
pub struct KernelStackFrame {
    // 0 - 255
    pub regs: [usize; 32],
    // 256 - 511
    pub fregs: [usize; 32],
    // 512
    pub pc: usize,
    // 520
    pub cpu_stack: usize,
    // 528
    pub user_frame: *mut TrapFrame,
}

const KERNEL_STACK_SIZE: usize = PAGE_SIZE - size_of::<usize>() - size_of::<KernelStackFrame>();

/// Kernel stack. The high memory stores `KernelStackFrame` to support context switching when running
/// in kernel mode.
#[repr(C)]
pub struct KernelStack {
    pub _stack: [u8; KERNEL_STACK_SIZE],
    pub reserved: usize,
    pub frame: KernelStackFrame
}

// Guard the size of `KernelStack` is PageSize.
sa::const_assert_eq!(size_of::<KernelStack>(), PAGE_SIZE);
