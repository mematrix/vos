//! CPU registers operations and data definitions of the RISC-V platform.

use core::arch::asm;


#[repr(usize)]
pub enum Register {
    Zero = 0,
    Ra,
    Sp,
    Gp,
    Tp,
    T0,
    T1,
    T2,
    S0,
    S1,
    /* 10 */
    A0,
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    A7,
    S2,
    S3,
    /* 20 */
    S4,
    S5,
    S6,
    S7,
    S8,
    S9,
    S10,
    S11,
    T3,
    T4,
    /* 30 */
    T5,
    T6,
}

pub const fn reg(r: Register) -> usize {
    r as usize
}

// Floating point registers
#[repr(usize)]
pub enum FRegister {
    Ft0,
    Ft1,
    Ft2,
    Ft3,
    Ft4,
    Ft5,
    Ft6,
    Ft7,
    Fs0,
    Fs1,
    /* 10 */
    Fa0,
    Fa1,
    Fa2,
    Fa3,
    Fa4,
    Fa5,
    Fa6,
    Fa7,
    Fs2,
    Fs3,
    /* 20 */
    Fs4,
    Fs5,
    Fs6,
    Fs7,
    Fs8,
    Fs9,
    Fs10,
    Fs11,
    Ft8,
    Ft9,
    /* 30 */
    Ft10,
    Ft11,
}

pub const fn freg(r: FRegister) -> usize {
    r as usize
}

////////////////// Supervisor CSRs R/W ////////////////////

/// Read `sstatus` register value.
pub fn sstatus_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr {}, sstatus", out(reg) rval);
        rval
    }
}

/// Write `val` to `sstatus` register.
pub fn sstatus_write(val: usize) {
    unsafe {
        asm!("csrw sstatus, {}", in(reg) val);
    }
}

/// Clear `SIE` bit of the `sstatus` register, return the **old** `sstatus` reg value. To restore the
/// old interrupt status, if no other operations changed the `sstatus` value, call [`sstatus_write`]
/// with the returned value.
///
/// [`sstatus_write`]: self::sstatus_write
pub fn sstatus_cli() -> usize {
    unsafe {
        let rd;
        // `sstatus` bit 1 -> sie
        asm!("csrrci {}, sstatus, 0b0010", out(reg) rd);
        rd
    }
}

/// Set `SIE` bit of the `sstatus` register. Not like the [`sstatus_cli`], this function does not
/// return the old `sstatus` reg value.
///
/// [`sstatus_cli`]: self::sstatus_cli
pub fn sstatus_sti() {
    unsafe {
        asm!("csrrsi x0, sstatus, 0b0010");
    }
}

pub fn sie_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr {}, sie", out(reg) rval);
        rval
    }
}

pub fn sie_write(val: usize) {
    unsafe {
        asm!("csrw sie, {}", in(reg) val);
    }
}

pub fn sscratch_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr {}, sscratch", out(reg) rval);
        rval
    }
}

pub fn sscratch_write(val: usize) {
    unsafe {
        asm!("csrw sscratch, {}", in(reg) val);
    }
}

/// Write `to` to the `sscratch` register and return the old value of the register.
pub fn sscratch_swap(to: usize) -> usize {
    unsafe {
        let from;
        asm!("csrrw {}, sscratch, {}", lateout(reg) from, in(reg) to);
        from
    }
}

pub fn sepc_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr {}, sepc", out(reg) rval);
        rval
    }
}

pub fn sepc_write(val: usize) {
    unsafe {
        asm!("csrw sepc, {}", in(reg) val);
    }
}

pub fn satp_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr {}, satp", out(reg) rval);
        rval
    }
}

pub fn satp_write(val: usize) {
    unsafe {
        asm!("csrw satp, {}", in(reg) val);
    }
}

/// Take a hammer to the page tables and synchronize all of them. This
/// essentially flushes the entire TLB.
pub fn satp_fense(vaddr: usize, asid: usize) {
    unsafe {
        asm!("sfence.vma {}, {}", in(reg) vaddr, in(reg) asid);
    }
}

/// Synchronize based on the address space identifier This allows us to
/// fence a particular process rather than the entire TLB.
pub fn satp_fense_asid(asid: usize) {
    unsafe {
        asm!("sfence.vma zero, {}", in(reg) asid);
    }
}

/////////////////// Performance Registers /////////////////

// todo: read the Supervisor shadow perf registers: time, cycle, etc.
