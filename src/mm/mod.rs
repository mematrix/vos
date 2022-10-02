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
mod vmem;

// Re-export
pub use vmem::*;

use core::arch::asm;


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


/// Init the physical memory management system, including the buddy allocator and the
/// `SLAB` allocator.
pub fn early_init(mem_regions: &[(usize, usize)]) {
    // First init the physical memory allocation system.
    page::init(mem_regions);
    // MMU API enable the page-based allocator feature.
    mmu::enable_page_allocator();

    // Init SLAB allocator for the kernel memory management.
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


/// The `SATP` register contains three fields: mode, address space id, and the first level table
/// address (level 2 for Sv39). This function helps make the 64-bit register contents based on
/// those three fields.
#[inline]
pub const fn build_satp(mode: mmu::Mode, asid: u64, addr: u64) -> usize {
    const ADDR_MASK: u64 = (1u64 << 44) - 1u64;
    (mode.val_satp() |
        (asid & 0xffff) << 44 |
        (addr >> 12) & ADDR_MASK) as usize
}
