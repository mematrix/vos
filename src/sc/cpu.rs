//! CPU and CPU-related routines.
//!
//! **Note**: After the `/driver/cpu` mod has been initialized, this mod becomes available.
//! The `SLAB` allocator must be initialized before init this mod.

use core::arch::asm;
use core::mem::size_of;
use core::ptr::{addr_of, null_mut};
use crate::mm::page::PAGE_SIZE;


/// Represents the CPU info.
#[repr(C)]
pub struct CpuInfo {
    /// CPU frequency.
    clock_freq: usize,
    /// CPU timebase frequency. We will perform the context switching per ~16ms (64 times per second),
    /// so the context switch time is `timebase_freq / 64`.
    timebase_freq: usize,
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
    pub fn set_clock_freq(&mut self, freq: usize) {
        self.clock_freq = freq;
    }

    #[inline(always)]
    pub fn get_clock_freq(&self) -> usize {
        self.clock_freq
    }

    #[inline(always)]
    pub fn set_timebase_freq(&mut self, freq: usize) {
        self.timebase_freq = freq;
    }

    #[inline(always)]
    pub fn get_timebase_freq(&self) -> usize {
        self.timebase_freq
    }

    /// Get the interval time (in CPU clocks) performing the context switching.
    #[inline(always)]
    pub fn get_ctx_switch_interval(&self) -> usize {
        self.timebase_freq / 64usize
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


/////////////////// CPU DATA /////////////////////////

static mut CPU_STACKS: *mut TrapStack = null_mut();
static mut CPU_COUNT: usize = 0;

/// Alloc and init the TrapStack memory for **per-cpu**.
pub fn init_per_cpu_data(cpu_count: usize) {
    unsafe {
        let cpus = crate::mm::early::alloc_obj::<TrapStack>(cpu_count);
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

pub fn get_info_by_cpuid(cpuid: usize) -> &'static mut CpuInfo {
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

pub fn get_stack_by_cpuid(cpuid: usize) -> &'static mut TrapStack {
    unsafe {
        debug_assert!(cpuid < CPU_COUNT);

        let cpu = CPU_STACKS.add(cpuid);
        &mut *cpu
    }
}

pub fn get_boot_cpu_stack() -> &'static mut TrapStack {
    unsafe {
        let count = CPU_COUNT;
        for id in 0..count {
            let cpu = CPU_STACKS.add(id);
            if (*cpu).info.get_hart_id() == 0 {
                return &mut *cpu;
            }
        }
    }
    panic!("Can not find the boot cpu (hart_id == 0) which is required.");
}

/// Get current hart's `CpuInfo` struct. Holding by the `tp` register.
pub fn current_cpu_info() -> &'static mut CpuInfo {
    unsafe {
        &mut *(crate::read_tp!() as *mut CpuInfo)
    }
}

pub fn current_cpu_frame() -> *const TrapStackFrame {
    unsafe {
        let info = crate::read_tp!() as *const CpuInfo;
        let stack = container_of!(info, TrapStack, info);
        addr_of!((*stack).frame)
    }
}
