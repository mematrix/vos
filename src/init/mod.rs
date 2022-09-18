//! Kernel initialization operation and data.

mod early_init;

use core::mem::size_of;
use fdt::Fdt;
use fdt::standard_nodes::Memory;
use crate::asm::mem_v::KERNEL_TABLE;
use crate::dev::{self, cpu};
use crate::mm::{self, create_kernel_identity_map, map_ram_region_identity};


pub const COMMAND_LINE_SIZE: usize = 256;

/// Untouched command line saved by arch-special code.
pub static mut BOOT_COMMAND_LINE: [u8; COMMAND_LINE_SIZE] = [0u8; COMMAND_LINE_SIZE];

extern "C" fn collect_memory_region(s_ptr: *mut u8, user_data: *const ()) -> *mut u8 {
    let memory = user_data as *const Memory;
    let memory = unsafe { &*memory };
    let pair = s_ptr as *mut (usize, usize);
    let mut idx = 0usize;
    for region in memory.regions() {
        if let Some(size) = region.size {
            // insert.
        }
    }
    unsafe { pair.add(idx).write((0, 0)); }

    // We **must** return the first param value.
    s_ptr
}

/// Setup on the early boot time.
/// Returns the SATP value (including the MODE).
pub fn early_setup(fdt: &Fdt) -> usize {
    let chosen = fdt.chosen();
    early_init::dt_scan_chosen(&chosen);

    // todo: move to `early_init` mod. use buddy allocator.
    let memory = fdt.memory();
    let reg_count = memory.regions().count();
    if reg_count == 0 {
        assert!(false, "No memory region");
    }

    let reg_ptr;
    unsafe {
        let mem_size = (reg_count + 1) * size_of::<usize>() * 2;
        let user_data = &memory as *const _ as *const ();
        // SAFETY: The callback func matches the requirement:
        //   - Returns the first param as the return value
        reg_ptr = mm::write_on_stack(mem_size, collect_memory_region, user_data);
    }

    let mut start_addr = 0usize;
    let mut mem_size = 0usize;
    // Init physical memory region
    for region in memory.regions() {
        if let Some(size) = region.size {
            start_addr = region.starting_address as usize;
            mem_size = size;
            // todo: currently we only handle the first memory region.
            break;
        }
    }

    mm::early_init(start_addr, mem_size);

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
    dev::init(4);
    let cpu = cpu::get_by_cpuid(0);
    cpu.set_hart_id(0);
    cpu.set_freq(10_000_000);   // QEMU frequency is 10MHz
}
