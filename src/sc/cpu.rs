//! CPU and CPU-related routines.
//!
//! **Note**: After the `/driver/cpu` mod has been initialized, this mod becomes available.
//! The `SLAB` allocator must be initialized before init this mod.

use core::arch::asm;
use core::mem::size_of;
use core::ptr::null_mut;
use crate::mm::{kmem::kzmalloc, page::PAGE_SIZE};


/// Represents the CPU info.
#[repr(C)]
pub struct CpuInfo {
    /// CPU frequency. We will perform the context switching per 10ms (100 times per second),
    /// so the context switch time is `freq / 100`.
    freq: u64,
    /// Cache the hart id, because the `mhartid` is a machine level CSR and we need the env-call
    /// to get the hart-id.
    hart_id: usize,
    // Extensions supported by the CPU.
    //extensions: usize,
}

impl CpuInfo {
    // We construct the `Cpu` object by performing a C-style cast from ptr instead of the usual
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

/// Context info for each **hart**.
#[repr(C)]
pub struct TrapStackFrame {
    /// `sp` register (`x2`). Stack frame pointer.
    pub sp: usize,
    /// `gp` register (`x3`). Global pointer, set to `_global_pointer` defined in the linker script.
    pub gp: usize,
    /// `tp` register (`x4`). Thread pointer, points to `CpuInfo` object of current **hart**.
    pub tp: usize,
}

const TRAP_STACK_SIZE: usize = PAGE_SIZE - size_of::<CpuInfo>() - size_of::<usize>() - size_of::<TrapStackFrame>();

/// Stack memory used in the trap handler. One-page size.
#[repr(C)]
pub struct TrapStack {
    _stack: [u8; TRAP_STACK_SIZE],
    pub reserved: usize,
    pub info: CpuInfo,
    pub frame: TrapStackFrame,
}

sa::const_assert_eq!(size_of::<TrapStack>(), PAGE_SIZE);

static mut CPU_STACKS: *mut TrapStack = null_mut();
static mut CPU_COUNT: usize = 0;

/// Alloc and init the TrapStack memory for **per-cpu**.
pub fn init_per_cpu_data(cpu_count: usize) {
    unsafe {
        let cpus = kzmalloc(PAGE_SIZE * cpu_count) as *mut TrapStack;
        assert!(!cpus.is_null());

        // Read the gp register value.
        let gp_val: usize;
        asm!("mv {}, gp", out(reg) gp_val);

        // Init frame of per cpu.
        for i in 0..cpu_count {
            let stack = &mut *cpus.add(i);
            stack.frame.sp = &stack.reserved as *const _ as usize;
            stack.frame.gp = gp_val;
            stack.frame.tp = &stack.info as *const _ as usize;
        }

        CPU_STACKS = cpus;
        CPU_COUNT = cpu_count;
    }
}

pub fn get_cpu_count() -> usize {
    unsafe {
        CPU_COUNT
    }
}

pub fn get_by_cpuid(cpuid: usize) -> &'static mut CpuInfo {
    unsafe {
        debug_assert!(cpuid < CPU_COUNT);

        let cpu = CPU_STACKS.add(cpuid);
        &mut (*cpu).info
    }
}

pub fn get_frame_by_cpuid(cpuid: usize) -> *const TrapStackFrame {
    unsafe {
        debug_assert!(cpuid < CPU_COUNT);

        let cpu = CPU_STACKS.add(cpuid);
        &(*cpu).frame as _
    }
}
