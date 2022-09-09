//! Kernel initialization operation and data.

mod early_init;

use fdt::Fdt;
use crate::mem::{self, create_kernel_identity_map, map_ram_region_identity};


pub const COMMAND_LINE_SIZE: usize = 256;

/// Untouched command line saved by arch-special code.
pub static mut BOOT_COMMAND_LINE: [u8; COMMAND_LINE_SIZE] = [0u8; COMMAND_LINE_SIZE];

/// Setup on the early boot time.
/// Returns the SATP value (including the MODE).
pub fn early_setup(fdt: &Fdt) -> usize {
    mem::init();

    let chosen = fdt.chosen();
    early_init::dt_scan_chosen(&chosen);

    let memory = fdt.memory();
    let id_map = create_kernel_identity_map();
    for region in memory.regions() {
        if let Some(size) = region.size {
            map_ram_region_identity(id_map, region.starting_address as usize, size);
        }
    }

    0
}
