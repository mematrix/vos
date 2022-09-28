//! Do initialization on boot time.

use fdt::standard_nodes::Memory;
use crate::constant::{ORDER_1GB, ORDER_2MB};
use crate::mm::virt_qemu;
use crate::mm::mmu::{create_root_table, EntryBits, Mode, Table};
use crate::util::align;


/// Build the identity page table. This table is used to map the virtual address
/// to the same physical address and is used only in the kernel space (S-mode).
///
/// **Note**: we map the identity PTE as a **Global** entry, so the kernel address
/// will be within a *small* range (limits to \[0, 2GiB + DRAM_SIZE]). To distinguish
/// the kernel address and the user address, for the Sv39 mode, the bit \[37] is set
/// to 0 with a *kernel address* while it is set to 1 with the *user address*.
/// According to the RISC-V Spec, the bits \[63:39] and bit \[38] must be equal
/// and we set it to 0.
pub fn build_kernel_identity_map(memory: &Memory) -> *mut dyn Table {
    // Construct the id map.
    let map_2mb = virt_qemu::get_mem_map_2mb();
    let map_1gb = virt_qemu::get_mem_map_1gb();
    let id_map = create_kernel_identity_map(map_2mb, map_1gb);
    for region in memory.regions() {
        if let Some(size) = region.size {
            let addr = region.starting_address as usize;
            map_ram_region_identity(id_map, addr, size);
        }
    }

    // Debug
    print_id_table_info(unsafe { &*id_map });

    id_map
}

// Show debug info.
fn print_id_table_info(root: &dyn Table) {
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
    let mode = root.get_mode().val_satp() as usize;
    println_k!("Root table mode: {:#x}", mode);
}


const ENTRY_LEVEL_2MB: u32 = 1;
const ENTRY_LEVEL_1GB: u32 = 2;

#[inline(always)]
fn map_identity<const ORDER: usize, const LEVEL: u32, const LENGTH: usize>(
    root: &mut dyn Table,
    maps: &[(usize, usize)],
    bits: u32) {
    for (mut start, size) in maps {
        let end = align::align_up(start + size, ORDER);
        while start < end {
            root.map(start, start, bits, LEVEL);
            start += LENGTH;
        }
    }
}

fn create_kernel_identity_map(map_2mb: &[(usize, usize)], map_1gb: &[(usize, usize)]) -> *mut dyn Table {
    let table = create_root_table(Mode::Sv39);

    // Sv39 mode:
    //   level 0 -> 4KiB per entry;
    //   level 1 -> 2MiB per entry;
    //   level 2 -> 1GiB per entry;

    // Ignore address [0, 2M), so the deref null pointer will fault as excepted.
    // Then map [2M, 1G) at level 1. And the following memory address will all
    // be mapped by the level 2 entry (1GiB per entry).
    //       root_table          l1_table(ppn=ppn[2]|ppn[1])
    //   [ 'branch entry' ] ---> [ 'Invalid' ]
    //   [  ppn[2] = 0x0  ]      [ ppn = 0x1 ]
    //   [  ppn[2] = 0x1  ]      [ ppn = 0x2 ]
    //   [  ppn[2] = 0x2  ]      [ ppn = 0x3 ]
    //   [  ppn[2] = 0x3  ]          .....
    //         ......            [ ppn = 511 ]

    // todo: handle EntryBits::Access & EntryBits::Dirty.

    let root = unsafe { &mut *table };
    // Map 2MiB page
    let bits = EntryBits::Access.val() | EntryBits::Dirty.val() |
        EntryBits::Global.val() | EntryBits::ReadWrite.val();
    const LENGTH_2MB: usize = 1usize << ORDER_2MB;
    map_identity::<ORDER_2MB, ENTRY_LEVEL_2MB, LENGTH_2MB>(root, map_2mb, bits);

    // Map 1GiB page
    const LENGTH_1GB: usize = 1usize << ORDER_1GB;
    map_identity::<ORDER_1GB, ENTRY_LEVEL_1GB, LENGTH_1GB>(root, map_1gb, bits);

    table
}

/// Map the DRAM region in the identity table. 1GB per entry, so the region \[addr:addr+len]
/// will first be aligned to 1GB boundary.
fn map_ram_region_identity(table: *mut dyn Table, addr: usize, len: usize) {
    // DRAM address should start from 0x8000_0000 (2G)
    debug_assert!(addr >= 0x8000_0000);

    // Map the DRAM space (2GiB - MemEnd)
    let bits = EntryBits::Access.val() | EntryBits::Dirty.val() |
        EntryBits::Global.val() | EntryBits::ReadWriteExecute.val();
    let mut start = align::align_down(addr, ORDER_1GB);
    let end = align::align_up(addr + len, ORDER_1GB);

    let root = unsafe { &mut *table };
    const LENGTH_1GB: usize = 1usize << ORDER_1GB;
    while start < end {
        root.map(start, start, bits, ENTRY_LEVEL_1GB);
        start += LENGTH_1GB;
    }
}
