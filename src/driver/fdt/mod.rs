//! Support to parsing Flatten Device Tree blob.

use fdt::{Fdt, node::FdtNode, standard_nodes::Chosen};
use crate::init::BOOT_COMMAND_LINE;

use core::ptr::copy_nonoverlapping;


/// Create a `Fdt` object from the dtb pointer.
#[inline]
pub unsafe fn parse_from_ptr<'a>(dtb: *const u8) -> Fdt<'a> {
    Fdt::from_ptr(dtb).expect("Device tree blob must be valid")
}

pub fn early_init_scan_chosen(chosen: &Chosen) {
    if let Some(args) = chosen.bootargs() {
        unsafe {
            copy_nonoverlapping(args.as_ptr(), BOOT_COMMAND_LINE.as_mut_ptr(), BOOT_COMMAND_LINE.len());
        }
    }
}

/// Show the standard nodes info of the `fdt`. Debug use only.
pub(crate) fn show_fdt_standard_nodes(fdt: &Fdt) {
    println_k!(" * Device tree total size: {} bytes.", fdt.total_size());
    println_k!(" * Root:");
    let root = fdt.root();
    println_k!("   * CellSize: {:?}", root.cell_sizes());
    println_k!("   * Model: {}", root.model());
    println_k!("   * Compatibility: {}", root.compatible().first());
    println_k!("   * Properties:");
    for p in root.properties() {
        println_k!("     * {}: str({}), usize({} ({2:#x}))", p.name,
                p.as_str().unwrap_or(""), p.as_usize().unwrap_or_default());
    }

    println_k!(" * Memory:");
    let memory = fdt.memory();
    if let Some(init_map) = memory.initial_mapped_area() {
        println_k!("   * initial-mapped-area: {:?}", init_map);
    }
    for r in memory.regions() {
        println_k!("   * region: starting_addr={:p}, size={:#x} ({1})",
                r.starting_address, r.size.unwrap_or_default());
    }

    println_k!(" * Chosen:");
    let chosen = fdt.chosen();
    println_k!("   * bootargs: {}", chosen.bootargs().unwrap_or("no args"));

    println_k!(" * Aliases:");
    if let Some(aliases) = fdt.aliases() {
        for a in aliases.all() {
            println_k!("   * {} -> {}", a.0, a.1);
        }
    }

    println_k!(" * CPUs:");
    for (id, cpu) in fdt.cpus().enumerate() {
        println_k!("   * cpu#{}:", id);
        println_k!("     * ids: {}", cpu.ids().first());
        for p in cpu.properties() {
            println_k!("     * {}: str({}), usize({} ({2:#x}))", p.name,
                    p.as_str().unwrap_or(""), p.as_usize().unwrap_or_default());
        }
    }
}

fn print_node(node: FdtNode<'_, '_>, n_spaces: usize) {
    (0..n_spaces).for_each(|_| print_k!(" "));
    println_k!("{}/", node.name);

    for child in node.children() {
        print_node(child, n_spaces + 2);
    }
}

pub(crate) fn dump_fdt(fdt: &Fdt) {
    println_k!();
    println_k!("////////// dump device tree (name) ////////////");
    if let Some(node) = fdt.find_node("/") {
        print_node(node, 0);
    }
    println_k!();
}
