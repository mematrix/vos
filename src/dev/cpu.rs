//! CPU and CPU-related routines. Provides the operations with the CSRs.
//!
//! **Note**: After the `/driver/cpu` mod has been initialized, this mod becomes available.
//! The `SLAB` allocator must be initialized before init this mod.

use core::arch::asm;
use core::mem::size_of;
use core::ptr::null_mut;
use crate::mem::{mmu::Mode, kmem::kzmalloc};


/// Represents the CPU info.
pub struct Cpu {
    /// CPU frequency. We will perform the context switching per 10ms (100 times per second),
    /// so the context switch time is `freq / 100`.
    freq: u64,
    /// Cache the hart id, because the `mhartid` is a machine level CSR and we need the env-call
    /// to get the hart-id.
    hart_id: usize,
    // Extensions supported by the CPU.
    //extensions: usize,
}

impl Cpu {
    // We don't construct the `Cpu` object by performing a C-style cast instead of the usual
    // constructor call, so no ctor method is provided.

    #[inline(always)]
    pub fn set_freq(&mut self, freq: u64) {
        self.freq = freq;
    }

    #[inline(always)]
    pub fn get_freq(&self) -> u64 {
        self.freq
    }

    /// Get the interval time (in CPU clocks) performing the context switching.
    #[inline(always)]
    pub fn get_ctx_switch_interval(&self) -> u64 {
        // or freq/128 ?
        self.freq / 100
    }

    #[inline(always)]
    pub fn set_hart_id(&mut self, hard_id: usize) {
        self.hart_id = hard_id;
    }

    #[inline(always)]
    pub fn get_hart_id(&self) -> usize {
        self.hart_id
    }
}

static mut CPU_INFOS: *mut Cpu = null_mut();
static mut CPU_COUNT: usize = 0;

/// Alloc the memory for all the info of `cpu_count` CPUs.
pub fn init_early_smp(cpu_count: usize) {
    unsafe {
        let cpus = kzmalloc(size_of::<Cpu>() * cpu_count) as *mut Cpu;
        assert!(!cpus.is_null());
        CPU_INFOS = cpus;
        CPU_COUNT = cpu_count;
    }
}

pub fn get_cpu_count() -> usize {
    unsafe {
        CPU_COUNT
    }
}

pub fn get_by_cpuid(cpuid: usize) -> &'static mut Cpu {
    unsafe {
        debug_assert!(cpuid < CPU_COUNT);

        let cpu = CPU_INFOS.add(cpuid);
        &mut *cpu
    }
}


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
    A0, /* 10 */
    A1,
    A2,
    A3,
    A4,
    A5,
    A6,
    A7,
    S2,
    S3,
    S4, /* 20 */
    S5,
    S6,
    S7,
    S8,
    S9,
    S10,
    S11,
    T3,
    T4,
    T5, /* 30 */
    T6
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
    Fa0, /* 10 */
    Fa1,
    Fa2,
    Fa3,
    Fa4,
    Fa5,
    Fa6,
    Fa7,
    Fs2,
    Fs3,
    Fs4, /* 20 */
    Fs5,
    Fs6,
    Fs7,
    Fs8,
    Fs9,
    Fs10,
    Fs11,
    Ft8,
    Ft9,
    Ft10, /* 30 */
    Ft11
}

pub const fn freg(r: FRegister) -> usize {
    r as usize
}

/// The `SATP` register contains three fields: mode, address space id, and the first level table
/// address (level 2 for Sv39). This function helps make the 64-bit register contents based on
/// those three fields.
#[inline]
pub const fn build_satp(mode: Mode, asid: u64, addr: u64) -> usize {
    const ADDR_MASK: u64 = (1u64 << 44) - 1u64;
    (mode.val_satp() |
        (asid & 0xffff) << 44 |
        (addr >> 12) & ADDR_MASK) as usize
}

////////////////// Supervisor CSRs R/W ////////////////////

pub fn sstatus_read() -> usize {
    unsafe {
        let rval;
        asm!("csrr {}, sstatus", out(reg) rval);
        rval
    }
}

pub fn sstatus_write(val: usize) {
    unsafe {
        asm!("csrw sstatus, {}", in(reg) val);
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
