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

pub(crate) mod page;
pub(crate) mod mmu;
pub(crate) mod kmem;

use mmu::{Table, Mode, EntryBits, create_root_table};


/// Returns the **aligned** value of `val`.
///
/// An **aligned** value is guaranteed that the least bits (width is specified
/// by `order`) are set to zero. Therefore, all alignments must be made as a
/// power of two.
///
/// This function always rounds up. So the returned value will always be
/// **not less than** the `val`.
pub const fn align_val(val: usize, order: usize) -> usize {
    let o = (1usize << order) - 1;
    (val + o) & !o
}

/// Returns the **aligned** value of `val`. Similar to [`align_val`], but this
/// function aligns value rounds down, it will simple set the least `order` bits
/// to zero. So the returned value will always be **not greater than** the `val`.
///
/// [`align_val`]: crate::mem::align_val
pub const fn align_down_val(val: usize, order: usize) -> usize {
    let o = (1usize << order) - 1;
    val & !o
}

/// Init the physical memory management property.
pub fn init() {
    // First init the physical pages
    page::init();

    // Init bytes-based allocator for the kernel memory management.
    kmem::init();
}

/* QEMU RISC-V memory maps (qemu/hw/riscv/virt.c)
static const MemMapEntry virt_memmap[] = {
    [VIRT_DEBUG] =        {        0x0,         0x100 },
    [VIRT_MROM] =         {     0x1000,        0xf000 },
    [VIRT_TEST] =         {   0x100000,        0x1000 },
    [VIRT_RTC] =          {   0x101000,        0x1000 },
    [VIRT_CLINT] =        {  0x2000000,       0x10000 },
    [VIRT_ACLINT_SSWI] =  {  0x2F00000,        0x4000 },
    [VIRT_PCIE_PIO] =     {  0x3000000,       0x10000 },
    [VIRT_PLATFORM_BUS] = {  0x4000000,     0x2000000 },
    [VIRT_PLIC] =         {  0xc000000, VIRT_PLIC_SIZE(VIRT_CPUS_MAX * 2) },
    [VIRT_APLIC_M] =      {  0xc000000, APLIC_SIZE(VIRT_CPUS_MAX) },
    [VIRT_APLIC_S] =      {  0xd000000, APLIC_SIZE(VIRT_CPUS_MAX) },
    [VIRT_UART0] =        { 0x10000000,         0x100 },
    [VIRT_VIRTIO] =       { 0x10001000,        0x1000 },
    [VIRT_FW_CFG] =       { 0x10100000,          0x18 },
    [VIRT_FLASH] =        { 0x20000000,     0x4000000 },
    [VIRT_IMSIC_M] =      { 0x24000000, VIRT_IMSIC_MAX_SIZE },
    [VIRT_IMSIC_S] =      { 0x28000000, VIRT_IMSIC_MAX_SIZE },
    [VIRT_PCIE_ECAM] =    { 0x30000000,    0x10000000 },
    [VIRT_PCIE_MMIO] =    { 0x40000000,    0x40000000 },
    [VIRT_DRAM] =         { 0x80000000,           0x0 },
};
*/

const VIRT_CPUS_MAX: usize = 1usize << 9;
const APLIC_MIN_SIZE: usize = 0x4000;
// const VIRT_IMSIC_MAX_SIZE: usize =

const fn aplic_size(cpus: usize) -> usize {
    APLIC_MIN_SIZE + align_val(cpus * 32, 14)
}

const fn virt_plic_size(cpus: usize) -> usize {
    const VIRT_PLIC_CONTEXT_BASE: usize = 0x200000;
    const VIRT_PLIC_CONTEXT_STRIDE: usize = 0x1000;

    VIRT_PLIC_CONTEXT_BASE + (cpus * 2) * VIRT_PLIC_CONTEXT_STRIDE
}

// 2M = 0x20_0000 = 1 << 21
/// Memory map list for page level 1 (2MiB per entry).
const VIRT_MEM_MAP: [(usize, usize); 10] = [
    // (0x100000, 0x2000),     // TEST and RTC. Lower than 2M, ignore to map
    (0x2000000, 0x10000),   // CLINT
    (align_down_val(0x2F00000, ORDER_2MB), 0x4000), // ACLINT_SSWI
    (0x3000000, 0x10000),   // PCIE_PIO
    (0x4000000, 0x2000000), // PLATFORM_BUS
    (0xc000000, virt_plic_size(VIRT_CPUS_MAX)), // PLIC
    (0xc000000, aplic_size(VIRT_CPUS_MAX)),     // APLIC_M
    (0xd000000, aplic_size(VIRT_CPUS_MAX)),     // APLIC_S
    (0x10000000, 0x102000),     // UART and VIRTIO and FW_CFG
    (0x20000000, 0x4000000),    // FLASH
    (0x30000000, 0x10000000),   // PCIE_ECAM
];

const ORDER_2MB: usize = 21;
const ORDER_1GB: usize = 30;
const ENTRY_LEVEL_2MB: u32 = 1;
const ENTRY_LEVEL_1GB: u32 = 2;

/// Create an identity page table. This table is used to map the virtual address
/// to the same physical address and is used only in the kernel space (S-mode).
///
/// **Note**: we map the identity PTE as a **Global** entry, so the kernel address
/// will be within a *small* range (limits to \[0, 2GiB + DRAM_SIZE]). To distinguish
/// the kernel address and the user address, for the Sv39 mode, the bit \[37] is set
/// to 0 with a *kernel address* while it is set to 1 with the *user address*.
/// According to the RISC-V Spec, the bits \[63:39] and bit \[38] must be equal
/// and we set it to 0.
pub fn create_kernel_identity_map() -> *mut dyn Table {
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

    let root = unsafe { &mut *table };
    // Map 2MiB page
    let bits = EntryBits::Global.val() | EntryBits::ReadWrite.val();
    for (mut start, size) in VIRT_MEM_MAP {
        let end = align_val(start + size, ORDER_2MB);
        while start < end {
            root.map(start, start, bits, ENTRY_LEVEL_2MB);
        }
    }

    // Map 1GiB-2GiB space (PCIE_MMIO)
    const ADDR_1G: usize = 1usize << ORDER_1GB; // 0x40000000;
    root.map(ADDR_1G, ADDR_1G, bits, ENTRY_LEVEL_1GB);

    table
}

/// Map the DRAM region in the identity table. 1GB per entry, so the region \[addr:addr+len]
/// will first be aligned to 1GB boundary.
pub fn map_ram_region_identity(table: *mut dyn Table, addr: usize, len: usize) {
    // DRAM address should start from 0x8000_0000 (2G)
    debug_assert!(addr >= 0x8000_0000);

    // Map the DRAM space (2GiB - MemEnd)
    let bits = EntryBits::Global.val() | EntryBits::ReadWriteExecute.val();
    let mut start = align_down_val(addr, ORDER_1GB);
    let end = align_val(addr + len, ORDER_1GB);

    let root = unsafe { &mut *table };
    const LENGTH_1GB: usize = 1usize << ORDER_1GB;
    while start < end {
        root.map(start, start, bits, ENTRY_LEVEL_1GB);
        start += LENGTH_1GB;
    }
}
