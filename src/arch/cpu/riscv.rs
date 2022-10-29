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

#[inline(always)]
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

#[inline(always)]
pub const fn freg(r: FRegister) -> usize {
    r as usize
}

////////////////////// CSR bits flag //////////////////////

/// `SPP` bit in `sstatus` register. The `sret` instruction will set the privilege level to
/// supervisor mode if `SPP` bit is 1, or user mode if `SPP` bit is 0.
pub const SSTATUS_SPP_BIT: usize = 1usize << 8;

////////////////////// Registers R/W //////////////////////

/// Read the `tp` register value.
#[macro_export]
macro_rules! read_tp {
    () => {{
        let tp: usize;
        ::core::arch::asm!("mv {}, tp", out(reg) tp, options(pure, nomem, nostack));
        tp
    }};
}

/// Write value to `tp` register.
#[macro_export]
macro_rules! write_tp {
    ($tp:expr) => {{
        let val: usize = $tp;
        ::core::arch::asm!("mv tp, {}", in(reg) val, options(nomem, nostack));
    }};
}

// pub macro write_tp($tp:expr) {}

//////////////////// Machine CSRs R/W /////////////////////

#[inline(always)]
pub fn mhartid_read() -> usize {
    unsafe {
        let id;
        asm!("csrr {}, mhartid", out(reg) id, options(pure, nomem, nostack));
        id
    }
}

////////////////// Supervisor CSRs R/W ////////////////////

/// Read `sstatus` register value.
#[inline(always)]
pub fn sstatus_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr {}, sstatus", out(reg) rval, options(pure, nomem, nostack));
        rval
    }
}

/// Write `val` to `sstatus` register.
#[inline(always)]
pub fn sstatus_write(val: usize) {
    unsafe {
        asm!("csrw sstatus, {}", in(reg) val, options(nomem, nostack));
    }
}

// `SIE` and `SPIE` bit are in the least 5 bits of `sstatus`, so we write the bits by single `csrrci`
// and `csrrsi` instructions. For other bits, we need a register value to clear and set the bits.

/// Clear the `SIE` of the `sstatus` register, return the **old** `sstatus` reg value. To restore the
/// old interrupt status, if no other operations changed the `sstatus` value, call [`sstatus_write`]
/// with the returned value.
///
/// [`sstatus_write`]: self::sstatus_write
#[inline(always)]
pub fn sstatus_cli() -> usize {
    unsafe {
        let rd;
        // `sstatus` bit 1 -> sie
        asm!("csrrci {}, sstatus, 0b0010", out(reg) rd, options(pure, nomem, nostack));
        rd
    }
}

/// Set the `SIE` bit of `sstatus` register. Not like the [`sstatus_cli`], this function does not
/// return the old `sstatus` reg value.
///
/// [`sstatus_cli`]: self::sstatus_cli
#[inline(always)]
pub fn sstatus_sti() {
    unsafe {
        asm!("csrrsi x0, sstatus, 0b0010", options(nomem, nostack));
    }
}

/// Set the `SPIE` bit of `sstatus` register.
#[inline(always)]
pub fn sstatus_set_spie() {
    unsafe {
        asm!("csrrsi x0, sstatus, 0b10000", options(nomem, nostack));
    }
}

// Clear the `SPIE` bit of `sstatus` register.
// pub fn sstatus_clear_spie() {
//     unsafe {
//         asm!("csrrci x0, sstatus, 0b10000");
//     }
// }

/// Set the bits of the `sstatus` register if the corresponding bits in `enable_bits` is 1.
///
/// See RISC-V CSR instruction `csrrs`.
#[inline(always)]
pub fn sstatus_set_bits(enable_bits: usize) {
    unsafe {
        asm!("csrrs x0, sstatus, {}", in(reg) enable_bits, options(nomem, nostack));
    }
}

/// Clear the bits of the `sstatus` register if the corresponding bits in `clear_bits` is 1.
///
/// See RISC-V CSR instruction `csrrc`.
#[inline(always)]
pub fn sstatus_clear_bits(clear_bits: usize) {
    unsafe {
        asm!("csrrc x0, sstatus, {}", in(reg) clear_bits, options(nomem, nostack));
    }
}

#[inline(always)]
pub fn sie_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr {}, sie", out(reg) rval, options(pure, nomem, nostack));
        rval
    }
}

#[inline(always)]
pub fn sie_write(val: usize) {
    unsafe {
        asm!("csrw sie, {}", in(reg) val, options(nomem, nostack));
    }
}

#[inline(always)]
pub fn sscratch_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr {}, sscratch", out(reg) rval, options(pure, nomem, nostack));
        rval
    }
}

#[inline(always)]
pub fn sscratch_write(val: usize) {
    unsafe {
        asm!("csrw sscratch, {}", in(reg) val, options(nomem, nostack));
    }
}

/// Write `to` to the `sscratch` register and return the old value of the register.
#[inline(always)]
pub fn sscratch_swap(to: usize) -> usize {
    unsafe {
        let from;
        asm!("csrrw {}, sscratch, {}", lateout(reg) from, in(reg) to, options(pure, nomem, nostack));
        from
    }
}

#[inline(always)]
pub fn sepc_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr {}, sepc", out(reg) rval, options(pure, nomem, nostack));
        rval
    }
}

#[inline(always)]
pub fn sepc_write(val: usize) {
    unsafe {
        asm!("csrw sepc, {}", in(reg) val, options(nomem, nostack));
    }
}

#[inline(always)]
pub fn satp_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr {}, satp", out(reg) rval, options(pure, nomem, nostack));
        rval
    }
}

#[inline(always)]
pub fn satp_write(val: usize) {
    unsafe {
        asm!("csrw satp, {}", in(reg) val, options(nomem, nostack));
    }
}

/// Take a hammer to the page tables and synchronize all of them. This
/// essentially flushes the entire TLB.
#[inline(always)]
pub fn satp_fense(vaddr: usize, asid: usize) {
    unsafe {
        asm!("sfence.vma {}, {}", in(reg) vaddr, in(reg) asid);
    }
}

/// Synchronize based on the address space identifier This allows us to
/// fence a particular process rather than the entire TLB.
#[inline(always)]
pub fn satp_fense_asid(asid: usize) {
    unsafe {
        asm!("sfence.vma zero, {}", in(reg) asid, options(nomem, nostack));
    }
}

/// Write `time` to `stimecmp` register.
#[inline(always)]
pub fn stimecmp_write(time: usize) {
    unsafe {
        asm!("csrw stimecmp, {}", in(reg) time, options(nomem, nostack));
    }
}

/// Read the `time` value, add with `delta`, then write the result to `stimecmp`.
#[inline(always)]
pub fn stimecmp_write_delta(delta: usize) {
    unsafe {
        asm!(
            "rdtime {tmp}",
            "add {tmp}, {tmp}, {delta}",
            "csrw stimecmp, {tmp}",
            delta = in(reg) delta,
            tmp = out(reg) _,
            options(nomem, nostack)
        );
    }
}

/////////////////// Performance Registers /////////////////

/// Read `time` register value.
#[inline(always)]
pub fn read_time() -> usize {
    unsafe {
        let t;
        asm!("rdtime {}", out(reg) t, options(pure, nomem, nostack));
        t
    }
}

// todo: read the Supervisor shadow perf registers: time, cycle, etc.

//////////////////// Other Instructions ///////////////////

/// Generate a `fence` instruction. If no order option is set, make a full fence on device "io"
/// and memory "rw".
#[macro_export]
macro_rules! fence {
    () => {
        fence!("iorw", "iorw");
    };
    ($p:expr, $s:expr) => {
        unsafe {
            // Add corresponding compiler fence call.
            ::core::arch::asm!(concat!("fence ", $p, ", ", $s), options(nomem, nostack));
        }
    }
}
