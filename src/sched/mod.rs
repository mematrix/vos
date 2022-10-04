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
//! simplify the asm code, the `cpu_stack` will point to the [`TrapStackFrame`] object which
//! is the inner object of the [`HartTrapStack`].
//!
//! The context registers:
//!
//! | Registers | Description |
//! | :-------: | ----------- |
//! | `x0`~`x31` | Generic registers. Note that `x0` is read-only constant zero. |
//! | `f0`~`f31` | Generic floating registers. More see **`*1*`** |
//! | `pc` | The instruction counter register. |
//!
//! > **`*1*`**: Note: The floating register need to be saved only when the `sstatus->FS`
//! field's value is 3.
//!
//! [`TaskTrapFrame`]: crate::proc::task::TaskTrapFrame
//! [`HartTrapStack`]: crate::smp::HartTrapStack
//! [`TrapStackFrame`]: crate::smp::TrapStackFrame

mod trap;
