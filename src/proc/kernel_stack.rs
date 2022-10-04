//! Kernel stack structs definition.

use core::mem::size_of;
use crate::mm::page::PAGE_SIZE;
use crate::proc::task::TaskTrapFrame;
use crate::smp::TrapStackFrame;


/// Kernel stack context frame. Used when interrupt is enabled while handling the `ecall`
/// sys-call to support single level recursive interrupt.
///
/// This is part definition of the *Full* kernel stack: only contains the high address of
/// the memory, and the low address range is used as the **stack** to run the sys-call func.
///
/// > **Note**: This stack is used only when the interrupt is enabled while we are handling
/// the sys-call. If the interrupt is disabled (This is the default setting when handling a
/// trap), we use the **Hart Trap Stack** binding to each hart as the function stack. See
/// [`TrapStack`].
///
/// The *Full* kernel stack is allocated as a single [page], so its size is 4KiB and the
/// available stack range is `[0, 4096 - sizeof::<KernelStack>() - 8]` (8bytes reserved).
/// See [`KernelStack`].
///
/// This definition has the same layout with the head part of [`TaskTrapFrame`], this can
/// simplify the context switch code.
///
/// **Note**: not like the `kernel_stack` field in [`TaskTrapFrame`], the `user_frame` (which
/// is in the same layout position as the `kernel_stack`) field points to the [`TaskTrapFrame`]
/// object's start address.
///
/// [page]: crate::mm::page
/// [`TrapStack`]: crate::smp::TrapStack
/// [`KernelStack`]: self::KernelStack
/// [`TaskTrapFrame`]: crate::proc::task::TaskTrapFrame
#[repr(C)]
pub struct KernelTrapFrame {
    // 0 - 255
    pub regs: [usize; 32],
    // 256 - 511
    pub fregs: [usize; 32],
    // 512
    pub pc: usize,
    // 520
    pub cpu_stack: *const TrapStackFrame,
    // 528
    pub user_frame: *mut TaskTrapFrame,
}

const KERNEL_STACK_SIZE: usize = PAGE_SIZE - size_of::<usize>() - size_of::<KernelTrapFrame>();

/// Kernel stack. The high memory stores [`KernelTrapFrame`] to support context switching when running
/// in kernel mode.
///
/// The size of this struct is exactly [`PAGE_SIZE`] bytes (4KiB). As the stack area is defined in low
/// memory, so the stack pointer should be `&reserved as *const ()` (The stack is grow from high addr
/// to the low addr).
///
/// [`KernelTrapFrame`]: self::KernelTrapFrame
/// [`PAGE_SIZE`]: crate::mm::page::PAGE_SIZE
#[repr(C)]
pub struct KernelStack {
    _stack: [u8; KERNEL_STACK_SIZE],
    pub reserved: usize,
    pub frame: KernelTrapFrame
}

// Guard the size of `KernelStack` is PageSize.
sa::const_assert_eq!(size_of::<KernelStack>(), PAGE_SIZE);
