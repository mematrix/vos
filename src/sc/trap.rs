//! Handle traps in Supervisor mode.

/// Rust trap handler. The `sscratch` register value need to keep unchanged before return.
///
/// Parameters are passed in from the asm code (`asm/trap.S`) by `a0`~`a5`:
///
/// - `a0`: `sepc` value, this is the virtual address of the instruction that was interrupted
/// or that encountered the exception.
/// - `a1`: `stval` value, saved the interrupt-associated value.
/// - `a2`: `scause` value, the cause of interrupt.
/// - `a3`: `sstatus` value.
/// - `a4`: `sscratch` value, points to the [`TrapFrame`] currently running.
/// - `a5`: Current hart's associated [`CpuInfo`].
///
/// This function returns the new `pc` value that continue to run after the trap returns.
/// - For interrupts, the return is usually input `a0` (`sepc` value).
/// - For exceptions (including `ecall`), we need to determine the next instruction address to
/// continue: for example, we should continue from the current `a0` address if exception is a
/// page fault exception; but we should continue from the next instruction address if exception
/// is raised by `ecall`, otherwise there will be a loop (return to `ecall` instruction and
/// trap again).
///
/// [`TrapFrame`]: crate::sc::TrapFrame
/// [`CpuInfo`]: crate::sc::cpu::CpuInfo
#[no_mangle]
extern "C"
fn handle_trap() -> usize {
    0
}
