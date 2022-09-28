//! Kernel memory management. Do the earlier memory initialization.
//!
//! Currently the kernel begun with an identity page map table, and the virtual
//! address is used as follows:
//!
//! | Addr Start | Size | Description |
//! | ---------- | ---- | ----------- |
//! | 0x00 | 2MiB | Unmap. Reserved. |
//! | 0x10_0000 | 2GiB - 2MiB | Miscellaneous IO devices, map to kernel. |
//! | 0x8000_0000 | DRAM_SIZE | Physical memory, map to kernel. |
//! | 0x20_0000_0000 | 128GiB | Map to user space. |
//! | 0x40_0000_0000 | To u64::max | Not used. |

pub(crate) mod early;
pub(crate) mod page;
pub(crate) mod mmu;
pub(crate) mod kmem;
pub(crate) mod virt_qemu;

use core::arch::asm;
use mmu::{create_root_table, EntryBits, Mode, Table};
use crate::util::align;


/// Heap area base address. Init before calling `early_init` and can not change after the
/// `early_init` call.
static mut HEAP_BASE: usize = 0;

/// Set the available heap base address.
///
/// **Note**: After calling the [`mm::early_init`] function, The heap base address must not
/// be changed.
pub fn set_heap_base_addr(heap_base: usize) {
    unsafe {
        debug_assert!(HEAP_BASE == 0usize);
        HEAP_BASE = heap_base;
    }
}


/// Init the physical memory management property.
pub fn early_init(mem_regions: &[(usize, usize)]) {
    // First init the physical pages
    page::init(mem_regions);

    // todo: move to kernel init phase.
    // Init bytes-based allocator for the kernel memory management.
    kmem::init();
}

/// Alloc a area on the stack. This will simple return the `sp` register value so the
/// returned ptr will be valid until the next function call.
///
/// **Note**: This allocation does not need a size param, the available memory area
/// depends on the stack size and current stack frame.
pub fn alloc_on_stack() -> *mut u8 {
    unsafe {
        let ret: usize;
        asm!("mv {}, sp", out(reg) ret);
        ret as *mut u8
    }
}

extern "C" {
    /// This is a **very dangerous** function, **The caller must guard that the callback func `cb`
    /// does not write out of bounds: at most `size` bytes is available**, otherwise the stack will
    /// be broken.
    pub fn write_on_stack(
        size: usize,
        cb: extern "C" fn(*mut u8, usize, *const ()),
        user_data: *const ()) -> *const u8;
}


// 2M = 0x20_0000 = 1 << 21
const ORDER_2MB: usize = 21;
const ORDER_1GB: usize = 30;
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

/// Create an identity page table. This table is used to map the virtual address
/// to the same physical address and is used only in the kernel space (S-mode).
///
/// **Note**: we map the identity PTE as a **Global** entry, so the kernel address
/// will be within a *small* range (limits to \[0, 2GiB + DRAM_SIZE]). To distinguish
/// the kernel address and the user address, for the Sv39 mode, the bit \[37] is set
/// to 0 with a *kernel address* while it is set to 1 with the *user address*.
/// According to the RISC-V Spec, the bits \[63:39] and bit \[38] must be equal
/// and we set it to 0.
pub fn create_kernel_identity_map(map_2mb: &[(usize, usize)], map_1gb: &[(usize, usize)]) -> *mut dyn Table {
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
pub fn map_ram_region_identity(table: *mut dyn Table, addr: usize, len: usize) {
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

/// The `SATP` register contains three fields: mode, address space id, and the first level table
/// address (level 2 for Sv39). This function helps make the 64-bit register contents based on
/// those three fields.
#[inline]
pub const fn build_satp(mode: Mode, asid: u64, addr: u64) -> usize {
    const ADDR_MASK: u64 = (1u64 << 44) - 1u64;
    (mode.val_satp() |
        (asid & 0xffff) << 44 |
        (addr >> 12) & ADDR_MASK) as usize
}
