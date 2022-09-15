//! Kernel initialization operation and data.

mod early_init;

use fdt::Fdt;
use crate::asm::mem_v::KERNEL_TABLE;
use crate::mm::{self, create_kernel_identity_map, map_ram_region_identity};


pub const COMMAND_LINE_SIZE: usize = 256;

/// Untouched command line saved by arch-special code.
pub static mut BOOT_COMMAND_LINE: [u8; COMMAND_LINE_SIZE] = [0u8; COMMAND_LINE_SIZE];

/// Setup on the early boot time.
/// Returns the SATP value (including the MODE).
pub fn early_setup(fdt: &Fdt) -> usize {
    let chosen = fdt.chosen();
    early_init::dt_scan_chosen(&chosen);

    let memory = fdt.memory();
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
    let id_map = create_kernel_identity_map();
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
