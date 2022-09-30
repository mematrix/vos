//! Kernel initialization operation and data.

mod boot_init;
mod early_init;

use core::mem::size_of;
use core::ptr::{copy_nonoverlapping, null, slice_from_raw_parts};
use fdt::standard_nodes::Memory;
use crate::asm::mem_v::KERNEL_TABLE;
use crate::driver::of;
use crate::mm;
use crate::sc;
use crate::util::align;


pub const COMMAND_LINE_SIZE: usize = 256;
/// Untouched command line saved by arch-special code.
pub static mut BOOT_COMMAND_LINE: [u8; COMMAND_LINE_SIZE] = [0u8; COMMAND_LINE_SIZE];


static mut DEVICE_TREE_BLOB: *const u8 = null();

/// Setup on boot time (Machine mode).
///
/// 1. Prepare kernel environment;
/// 2. Parse the special DeviceTree node from the `boot_dtb` passed by the firmware;
/// 3. Copy `boot_dtb` to kernel memory for full parsing later;
/// 4. Init per-cpu stack data;
/// 5. Build the identity map for S-mode kernel address translation.
/// 6. Early smp setup, init the hart environment and prepare to run into kernel.
pub fn boot_setup(boot_dtb: *const u8) -> usize {
    // todo: init uart using the info from dtb.
    let uart = crate::driver::uart::Uart::default();
    uart.init_default();

    // Set the heap base address.
    mm::set_heap_base_addr(unsafe { crate::asm::mem_v::HEAP_START });

    let fdt = unsafe { of::fdt::parse_from_ptr(boot_dtb) };
    of::fdt::show_fdt_standard_nodes(&fdt);
    of::fdt::dump_fdt(&fdt);

    // Copy dtb from the boot memory to the kernel memory.
    let dtb_size = fdt.total_size();
    let bytes = mm::early::alloc_bytes(dtb_size);
    unsafe {
        copy_nonoverlapping(boot_dtb, bytes, dtb_size);
        DEVICE_TREE_BLOB = bytes as _;
    }

    // Parse CPU node and prepare per-cpu stack.
    let cpu_count = fdt.cpus().count();
    sc::boot_init(cpu_count);
    for (idx, cpu_node) in fdt.cpus().enumerate() {
        let cpu = sc::cpu::get_info_by_cpuid(idx);
        // `clock_frequency` is not provided on risc-v cpu node.
        // cpu.set_clock_freq(cpu_node.clock_frequency());
        cpu.set_timebase_freq(cpu_node.timebase_frequency());
        cpu.set_hart_id(cpu_node.ids().first());
    }

    // Set boot cpu (current cpu) env.
    let boot_cpu = sc::cpu::get_boot_cpu_stack();
    unsafe { crate::write_tp!(boot_cpu.frame.tp); }

    // Build kernel identity map.
    let memory = fdt.memory();
    let id_map = boot_init::build_kernel_identity_map(&memory);

    // Build SATP value and return.
    let root = unsafe { &*id_map };
    let addr = root.get_addr();
    unsafe {
        KERNEL_TABLE = addr;
    }
    mm::build_satp(root.get_mode(), 0, addr as u64)

    // On this time, kernel identity map is already built.
    // todo: smp::boot_setup wake up other CPUs to do boot init.
}

/// Setup on the boot CPU (hart id == 0) when the kernel start.
///
/// 1. Init the physical memory management subsystem.
/// 2. Register all kernel built-in drivers.
/// 3. Un-flatten the DeviceTree to the runtime object, then build the kernel device tree and probe
/// the device drivers.
/// 4. Init file system.
/// 5. Load drivers from disk and probe.
/// 6. Standard I/O setup, GPU init, mouse/keyboard init, PIC (Platform Interrupt Control) init, etc.
/// 7. Prepare the environment for running the kernel thread and user process (smp setup, scheduler
/// init, process static data init, etc).
pub fn kernel_setup() {
    let fdt = unsafe { of::fdt::parse_from_ptr::<'static>(DEVICE_TREE_BLOB) };
    let chosen = fdt.chosen();
    early_init::dt_scan_chosen(&chosen);

    let memory = fdt.memory();
    let reg_count = memory.regions().count();
    assert!(reg_count > 0, "No memory region");

    // Init physical memory region
    unsafe {
        // We allocate space for two additional entries: one for the finish entry(not used currently);
        // and another for the alignment to satisfy the request of rust borrow variable and ptr.read().
        let mem_size = (reg_count + 2) * size_of::<(usize, usize)>();
        let user_data = &memory as *const _ as *const ();
        // SAFETY: The callback func matches the requirement:
        //   - Write at most `mem_size` bytes (guard by the assert).
        mm::write_on_stack(mem_size, collect_memory_region_and_init, user_data);
    }

    // Debug output
    mm::page::print_page_allocations();

    // todo: init slab

}


/// Collect the memory regions from the DeviceTree and do early mm init.
extern "C" fn collect_memory_region_and_init(s_ptr: *mut u8, count: usize, user_data: *const ()) {
    let memory = user_data as *const Memory;
    let memory = unsafe { &*memory };
    // The stack pointer may not satisfy the alignment.
    let pair = align::align_up_of::<(usize, usize)>(s_ptr as usize);
    let pair = pair as *mut (usize, usize);
    let mut idx = 0usize;
    for region in memory.regions() {
        if let Some(size) = region.size {
            if size == 0usize {
                continue;
            }
            // insert.
            let addr = region.starting_address as usize;
            let mut ins_pos = idx;
            while ins_pos > 0usize {
                let (a, s) = unsafe { pair.add(ins_pos - 1usize).read() };
                if addr >= a {
                    break;
                }
                unsafe { pair.add(ins_pos).write((a, s)); }
                ins_pos -= 1usize;
            }
            unsafe { pair.add(ins_pos).write((addr, size)); }

            idx += 1;
        }
    }
    assert!((idx + 1) * size_of::<(usize, usize)>() <= count);

    let regions = if idx <= 1usize {
        slice_from_raw_parts(pair, idx)
    } else {
        let total = idx;
        idx = 1usize;
        let mut seq_idx = 0usize;
        let (mut seq_ptr, mut seq_size) = unsafe { pair.add(seq_idx).read() };
        // coalesce.
        while idx < total {
            let (ptr, size) = unsafe { pair.add(idx).read() };
            if seq_ptr + seq_size == ptr {
                // Continuous
                seq_size += size;
            } else if seq_ptr + seq_size > ptr {
                // Memory region overlapped
                warn!("Memory region overlapped: [{:#x}, {:#x}] and [{:#x}, {:#x}].",
                    seq_ptr, seq_ptr + seq_size, ptr, ptr + size);
                if seq_ptr + seq_size < ptr + size {
                    seq_size = ptr + size - seq_ptr;
                }
            } else {
                // Segment
                unsafe { pair.add(seq_idx).write((seq_ptr, seq_size)); }
                seq_idx += 1usize;
                seq_ptr = ptr;
                seq_size = size;
            }
            idx += 1usize;
        }

        unsafe { pair.add(seq_idx).write((seq_ptr, seq_size)); }
        seq_idx += 1usize;
        slice_from_raw_parts(pair, seq_idx)
    };

    mm::early_init(unsafe { &*regions });
}
