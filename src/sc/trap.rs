//! Handle traps in Supervisor mode.

use crate::sc::cpu::CpuInfo;
use crate::sc::TrapFrame;


/// Check the `SPP` field of `sstatus`, return true if Previous Privilege is S-mode.
///
/// The `SPP` bit indicates the privilege level at which a hart was executing before entering
/// supervisor mode. When a trap is taken, `SPP` is set to 0 if the trap originated from user
/// mode, or 1 otherwise.
#[inline(always)]
fn trap_from_s_mode(status: usize) -> bool {
    // SPP in bit 8.
    status & 0b1_0000_0000 != 0
}

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
fn handle_trap(
    epc: usize, val: usize, cause: usize, status: usize,
    frame: &mut TrapFrame,
    hart: &CpuInfo) -> usize {
    // The cause contains the type of trap (sync, async) as well as the cause number.
    // The most significant bit (aka `Interrupt bit`) is set if the trap was caused by an interrupt.
    let is_async = (cause as isize).is_negative();

    let exp_code = (cause << 1) >> 1;
    let mut return_pc = epc;
    if is_async {
        // Interrupt.
        match exp_code {
            1 => {
                // Supervisor software interrupt.
                // We will use this interrupt to waken our CPUs so that they can process processes.
                debug!("Supervisor software interrupt on hart #{}", hart.get_hart_id());
            }
            5 => {
                // Supervisor timer interrupt.
                // Do context switching.
                trace!("Supervisor timer interrupt on hart #{}", hart.get_hart_id());
            }
            9 => {
                // Supervisor external interrupt.
                trace!("Supervisor external interrupt on hart #{}", hart.get_hart_id());
            }
            _ => {
                // Unhandled/Unexpected interrupts.
                let hart_id = hart.get_hart_id();
                panic!("Unhandled interrupts on hart #{}, exp code: {}", hart_id, exp_code);
            }
        }
    } else {
        // Exception.
        match exp_code {
            0 | 1 | 2 => {
                // 0: Instruction address misaligned.
                // 1: Instruction access fault.
                // 2: Illegal Instruction.
                if trap_from_s_mode(status) {
                    // S-mode code exception.
                    panic!("Instruction exception, code: {}, epc: {:#x}, trap val: {}.",
                           exp_code, epc, val);
                }

                error!("Instruction exception with PID {}, exp code: {}. epc: {:#x}, trap val: {}.",
                    frame.pid, exp_code, epc, val);
                // Close the exception process, re-schedule.
            }
            3 => {
                // Breakpoint.
                debug!("Breakpoint on hart #{}, pc @{:#x}", hart.get_hart_id(), epc);
                return_pc += 2;
            }
            4 | 5 | 6 | 7 => {
                // 4: Load address misaligned.
                // 5: Load access fault.
                // 6: Store/AMO address misaligned.
                // 7: Store/AMO access fault.
                if trap_from_s_mode(status) {
                    panic!("Memory access exception, code: {}, epc: {:#x}, trap val: {}.",
                           exp_code, epc, val);
                }

                error!("Memory access exception with PID {}, exp code: {}. epc: {:#x}, trap val: {}.",
                    frame.pid, exp_code, epc, val);
                // Close the exception process, re-schedule.
            }
            8 => {
                // Environment call from U-mode.
                debug!("Env call from PID {}.", frame.pid);
                return_pc += 4;
            }
            12 | 13 | 15 => {
                // 12: Instruction page fault.
                // 13: Load page fault.
                // 15: Store/AMO page fault.
                error!("Page fault. exp code: {}, epc: {:#x}, trap val: {}.",
                    exp_code, epc, val);
                return_pc += 4;
                // todo: swap page. keep return pc unchanged.
            }
            _ => {
                // Unhandled exceptions.
                let hart_id = hart.get_hart_id();
                panic!("Unhandled exception on hart #{}, exp code: {}, pc @{:#x}, trap val: {:#x}.",
                       hart_id, exp_code, epc, val);
            }
        }
    }

    return_pc
}
