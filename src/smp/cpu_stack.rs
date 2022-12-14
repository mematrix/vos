//! CPU based context info.

use core::arch::asm;
use core::mem::size_of;
use core::ptr::{addr_of, null_mut};
use crate::mm::PAGE_SIZE;
use crate::smp::{CPU_COUNT, CpuInfo};


/// Context info for each **hart**.
#[repr(C)]
pub struct HartFrameInfo {
    /// `sp` register (`x2`). Stack frame pointer.
    pub sp: usize,
    /// `gp` register (`x3`). Global pointer, set to `_global_pointer` defined in the linker script.
    pub gp: usize,
    /// `tp` register (`x4`). Thread pointer, points to `CpuInfo` object of current **hart**.
    pub tp: usize,
}

const TRAP_STACK_SIZE: usize = PAGE_SIZE - size_of::<CpuInfo>() - size_of::<usize>()
    - size_of::<HartFrameInfo>();

/// Stack memory used in the trap handler. One-page size.
#[repr(C)]
pub struct HartTrapStack {
    _stack: [u8; TRAP_STACK_SIZE],
    pub reserved: usize,
    pub info: CpuInfo,
    pub frame: HartFrameInfo,
}

sa::const_assert_eq!(size_of::<HartTrapStack>(), PAGE_SIZE);


/////////////////// CPU DATA /////////////////////////

static mut CPU_STACKS: *mut HartTrapStack = null_mut();

/// Alloc and init the HartTrapStack memory for **per-cpu**.
pub(super) fn init_per_cpu_stack(cpu_count: usize) {
    unsafe {
        let cpus = crate::mm::early::alloc_obj::<HartTrapStack>(cpu_count);
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
    }
}


/// Get mut `CpuInfo` object by the `cpuid`.
pub fn get_cpu_info_by_cpuid_mut(cpuid: usize) -> &'static mut CpuInfo {
    unsafe {
        debug_assert!(cpuid < CPU_COUNT);

        let cpu = CPU_STACKS.add(cpuid);
        &mut (*cpu).info
    }
}

/// Get `CpuInfo` object by the `cpuid`.
pub fn get_cpu_info_by_cpuid(cpuid: usize) -> &'static CpuInfo {
    unsafe {
        debug_assert!(cpuid < CPU_COUNT);

        let cpu = CPU_STACKS.add(cpuid);
        &(*cpu).info
    }
}

/// Get `HartFrameInfo` object by the `cpuid`.
pub fn get_cpu_frame_by_cpuid(cpuid: usize) -> *const HartFrameInfo {
    unsafe {
        debug_assert!(cpuid < CPU_COUNT);

        let cpu = CPU_STACKS.add(cpuid);
        addr_of!((*cpu).frame)
    }
}

/// Get mut `HartTrapStack` object by the `cpuid`.
pub fn get_cpu_stack_by_cpuid_mut(cpuid: usize) -> &'static mut HartTrapStack {
    unsafe {
        debug_assert!(cpuid < CPU_COUNT);

        let cpu = CPU_STACKS.add(cpuid);
        &mut *cpu
    }
}

/// Get cpu stack of boot cpu (hart id == 0).
pub fn get_boot_cpu_stack() -> &'static mut HartTrapStack {
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

/// Get current hart's `HartFrameInfo` object.
pub fn current_cpu_frame() -> *const HartFrameInfo {
    unsafe {
        let info = crate::read_tp!() as *const CpuInfo;
        let stack = container_of!(info, HartTrapStack, info);
        addr_of!((*stack).frame)
    }
}
