//! Kernel initialization operation and data.

mod early_init;

use core::mem::size_of;
use core::ptr::slice_from_raw_parts;
use fdt::Fdt;
use fdt::standard_nodes::Memory;
use crate::asm::mem_v::KERNEL_TABLE;
use crate::mm::{self, create_kernel_identity_map, map_ram_region_identity};
use crate::sc::{self, cpu};
use crate::util::align;


pub const COMMAND_LINE_SIZE: usize = 256;

/// Untouched command line saved by arch-special code.
pub static mut BOOT_COMMAND_LINE: [u8; COMMAND_LINE_SIZE] = [0u8; COMMAND_LINE_SIZE];

pub fn boot_setup() {
    // Set the heap base address.
    mm::set_heap_base_addr(unsafe { crate::asm::mem_v::HEAP_START });

    sc::boot_init(4);
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

/// Setup on the early boot time.
/// Returns the SATP value (including the MODE).
pub fn early_setup(fdt: &Fdt) -> usize {
    let chosen = fdt.chosen();
    early_init::dt_scan_chosen(&chosen);

    // todo: move to `early_init` mod. use buddy allocator.
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

    // Construct the id map.
    let map_2mb = mm::virt_qemu::get_mem_map_2mb();
    let map_1gb = mm::virt_qemu::get_mem_map_1gb();
    let id_map = create_kernel_identity_map(map_2mb, map_1gb);
    for region in memory.regions() {
        if let Some(size) = region.size {
            let addr = region.starting_address as usize;
            map_ram_region_identity(id_map, addr, size);
        }
    }

    // Debug output
    mm::page::print_page_allocations();

    let root = unsafe { &*id_map };
    // Test address translation
    let va = 0x8000_8a86usize;
    let pa = root.virt_to_phys(va);
    if let Some(pa) = pa {
        println_k!("Walk va {:#x} = pa {:#x}", va, pa);
    } else {
        println_k!("Test: Could not translate va {:#x} to pa.", va);
    }

    let addr = root.get_addr();
    println_k!("Root table addr: {:#x}", addr);
    unsafe {
        KERNEL_TABLE = addr;
    }
    let mode = root.get_mode().val_satp() as usize;
    println_k!("Root table mode: {:#x}", mode);

    mode | (addr >> 12)
}

/// Setup when the kernel start. Init the `SLAB` allocator; un-flatten the DeviceTree to
/// the runtime object; init the kernel data view for specific devices; register device
/// drivers; create devices and probe the drivers.
pub fn setup() {
    // todo: init slab

    // todo: read cpu count from DeviceTree.
    // sc::init(4);
    let cpu = cpu::get_by_cpuid(0);
    cpu.set_hart_id(0);
    cpu.set_freq(10_000_000);   // QEMU frequency is 10MHz
}
