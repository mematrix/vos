//! CPU and CPU-related routines. Provides the operations with the CSRs.
//!
//! **Note**: After the `/driver/cpu` mod has been initialized, this mod becomes available.
//! The `SLAB` allocator must be initialized before init this mod.

use core::mem::size_of;
use core::ptr::null_mut;
use crate::mem::{kmem::kzmalloc};


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

