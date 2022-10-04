//! MMU operations. Support Sv39, Sv48, and Sv57 mode. Sv32 mode is used in RV32
//! but the main structure and operations is similar.
//!
//! # Memory allocator
//!
//! On boot time, the page-based allocator has not been inited so we need to use
//! the [early allocator API] to alloc page-size memory for the PageTable. After
//! kernel initialized the page-based allocator (aka buddy allocator) system, the
//! [`enable_page_allocator`] **must** be called to update the allocator used.
//!
//! # Note
//!
//! Any MMU function **must** be called from the M-mode or in the S-mode with an
//! identity PTE is set to cover the PageEntry physical memory. Because read and
//! write operation of the PTE all access the physical memory directly.
//!
//! [early allocator API]: crate::mm::early
//! [`enable_page_allocator`]: self::enable_page_allocator

use core::ptr::null_mut;
use crate::mm::{PAGE_ORDER, PAGE_SIZE};


/// Delegate allocator API for the `mmu` mod.
mod allocator {
    use crate::mm::early::alloc_bytes_aligned;
    use crate::mm::page::{self};
    use crate::mm::{PAGE_ORDER, PAGE_SIZE};

    fn early_alloc_page() -> usize {
        alloc_bytes_aligned(PAGE_SIZE, PAGE_ORDER) as usize
    }

    fn early_dealloc_page(_addr: usize) {}

    fn kernel_alloc_page() -> usize {
        page::alloc_page(0)
    }

    fn kernel_dealloc_page(addr: usize) {
        page::free_page(addr);
    }

    static mut ALLOC_FN: fn() -> usize = early_alloc_page;
    static mut DEALLOC_FN: fn(usize) = early_dealloc_page;

    pub fn alloc_page() -> usize {
        unsafe { ALLOC_FN() }
    }

    pub fn free_page(addr: usize) {
        unsafe { DEALLOC_FN(addr); }
    }

    pub fn alloc_zeroed_page() -> usize {
        let addr = alloc_page();
        if addr != 0 {
            // We got a block of 4094 bytes (page size).
            let big_ptr = addr as *mut u64;
            unsafe {
                // SIMD can be used?
                big_ptr.write_bytes(0, PAGE_SIZE / 8usize);
            }
        }

        addr
    }

    /// Enable the page-based allocator. After this call, all memory alloc operations
    /// in this mod will delegate to the page-based allocator (buddy allocator). This
    /// function should be called **only once** after the buddy allocator system had
    /// been inited and before any MMU API is called.
    pub fn enable_page_allocator() {
        unsafe {
            ALLOC_FN = kernel_alloc_page;
            DEALLOC_FN = kernel_dealloc_page;
        }
    }
}

pub use self::allocator::enable_page_allocator;


#[repr(u32)]
#[derive(Copy, Clone)]
/// Entry flag bits. Represent as unsigned 32-bit integer.
pub enum EntryBits {
    None = 0,
    Valid = 1 << 0,
    Read = 1 << 1,
    Write = 1 << 2,
    Execute = 1 << 3,
    User = 1 << 4,
    Global = 1 << 5,
    Access = 1 << 6,
    Dirty = 1 << 7,

    // Convenience combinations
    ReadWrite = 1 << 1 | 1 << 2,
    ReadExecute = 1 << 1 | 1 << 3,
    ReadWriteExecute = 1 << 1 | 1 << 2 | 1 << 3,

    // User Convenience Combinations
    UserReadWrite = 1 << 1 | 1 << 2 | 1 << 4,
    UserReadExecute = 1 << 1 | 1 << 3 | 1 << 4,
    UserReadWriteExecute = 1 << 1 | 1 << 2 | 1 << 3 | 1 << 4,
}

impl EntryBits {
    #[inline]
    pub const fn val(self) -> u32 {
        self as u32
    }

    #[inline]
    pub const fn val_u64(self) -> u64 {
        self as u32 as u64
    }
}

/// Check if the EntryBits combination is valid.
///
/// We represent the EntryBits as a u32 type, but only the least 8 bits is
/// useful. And for the permission R,W,X bits, **Writable pages must also be
/// marked readable**, the contrary combinations are reserved for future use.
/// So the Entry bits is invalid if `bits & 0x06 == 0x4`
/// (`bits & 0b0110 == 0b0100`).
#[inline]
const fn is_bits_valid(bits: u32) -> bool {
    bits & 0b0110u32 != 0b0100u32
}

/// Check if the EntryBits is valid and is a leaf PTE.
///
/// Leaf PTE has a EntryBits that at least one bit of R,W,X has been set.
#[inline]
const fn is_bits_valid_leaf(bits: u32) -> bool {
    is_bits_valid(bits) && (bits & 0b1110u32 != 0u32)
}

// For all Sv32, Sv39, Sv48, Sv57 modes, the least 10 bits [9:0] have the same
// meaning:
// Flags:  9...8 7 6 5 4 3 2 1 0
//         [RSW] D A G U X W R V
// For Sv32 mode, the most 22 bits [31:10] hold the `ppn` (Physical Page Number).
// For Sv39, Sv48, Sv57 modes, the most 10 bits [63:54] are reserved or for
// extensions, and should be set to zero; the bits [53:10] hold the `ppn`.
// Sv39:   53  ..............................  28|27  ....  19|18  ....  10
//         [               PPN[2]               ]|[  PPN[1]  ]|[  PPN[0]  ]
// Sv48:   53  .................  37|36  ....  28|27  ....  19|18  ....  10
//         [         PPN[3]        ]|[  PPN[2]  ]|[  PPN[1]  ]|[  PPN[0]  ]
// Sv57:   53  ....  46|45  ....  37|36  ....  28|27  ....  19|18  ....  10
//         [  PPN[4]  ]|[  PPN[3]  ]|[  PPN[2]  ]|[  PPN[1]  ]|[  PPN[0]  ]

// The Sv39, Sv48, Sv57 modes support a 39-bit, 48-bit, 57-bit virtual address
// respectively. That is say not all bits in the virtual address is useful, so
// we need map the narrower virtual address (39 bits usable space for the Sv39
// mode) to a 64-bit virtual address value.
// Instead of zero-extension, a rule similar to the sign-extension is used:
// The most-significant bits of the full-size (64-bit) must be the same as the
// most-significant bit of the usable space (39 bits for Sv39 mode).
// For example, for Sv39 mode, the bits [63:39] must all equal to bit 38.
// This allows an OS to use one or a few of the most-significant bits of a
// full-size (64-bit) virtual address to quickly distinguish user and supervisor
// address regions.
// But for the physical address, zero-extension is used from a narrower physical
// address to a wider size.

/// A single **page table entry** (PTE) for the RV64 system.
///
/// The page table entry is described in the **RISC-V Privileged Architecture**
/// Chapter 4.3 - 4.6.
///
/// > RISC-V specified multiple virtual memory systems for RV64 to relieve the
/// tension between providing a large address space and minimizing address-translation
/// cost. For many systems, 512 GiB of virtual-address space is ample, and so
/// Sv39 suffices. Sv48 increases the virtual address space to 256 TiB, but
/// increases the physical memory capacity dedicated to page tables, the latency
/// of page-table traversals, and the size of hardware structures that store virtual
/// addresses. Sv57 increases the virtual address space, page table capacity
/// requirement, and translation latency even further.
struct Entry {
    entry: u64,
}

impl Entry {
    /// Check if the **Valid** bit is set.
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.entry & EntryBits::Valid.val_u64() != 0
    }

    /// Check if the **Valid** bit is clear.
    #[inline]
    pub fn is_invalid(&self) -> bool {
        !self.is_valid()
    }

    /// Check if this entry is a leaf. A leaf has one or more RWX bits set.
    #[inline]
    pub fn is_leaf(&self) -> bool {
        self.entry & EntryBits::ReadWriteExecute.val_u64() != 0
    }

    /// Check if this entry is a branch. A branch is that RWX bits are all zero.
    #[inline]
    pub fn is_branch(&self) -> bool {
        !self.is_leaf()
    }

    /// Check if the **Access** bit is set.
    #[inline]
    pub fn is_access_set(&self) -> bool {
        self.entry & EntryBits::Access.val_u64() != 0
    }

    /// Check if the **Access** bit is clear.
    #[inline]
    pub fn is_access_clear(&self) -> bool {
        !self.is_access_set()
    }

    #[inline]
    pub fn set_entry(&mut self, entry: u64) {
        self.entry = entry;
    }

    #[inline]
    pub fn get_entry(&self) -> u64 {
        self.entry
    }
}

/// The address-translation schema that a RV64 system supports.
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum Mode {
    Bare = 0,
    Sv39 = 8,
    Sv48 = 9,
    Sv57 = 10,
}

impl Mode {
    #[inline]
    pub const fn val(self) -> u8 {
        self as u8
    }

    /// Convenience function to make the **MODE** representation in the `satp`
    /// register. The mode value has been left shift to the bits \[63:60].
    #[inline]
    pub const fn val_satp(self) -> u64 {
        (self.val() as u64) << 60
    }
}

/// Operations of the page table.
///
/// **Note**: All method call of this trait **must** be in the M-mode or the
/// S-mode with an identity PTE which covers all physical memory that the page
/// table entries store.
pub trait Table {
    /// Get the **physical address** that the Table is allocated in. This value
    /// is usually used to build the `ppn` of the `satp` register.
    fn get_addr(&self) -> usize;

    /// Get the address-translation schema that the Table used.
    fn get_mode(&self) -> Mode;

    /// Map a virtual address to a physical address.
    ///
    /// The map page size is controlled by the `level`: level 0 means the lowest
    /// level refers to the 4KiB pages; level 1 refers to the 2MiB *megapages*,
    /// and etc. **each of the level must be virtually and physically aligned to
    /// a boundary equal to its size.**
    fn map(&mut self, v_addr: usize, p_addr: usize, bits: u32, level: u32);

    /// Unmap the virtual address from the page table.
    ///
    /// Returns `true` if the PTE was changed (when unmap success), otherwise
    /// returns `false`.
    fn unmap(&mut self, v_addr: usize) -> bool;

    /// Walk the page table to convert a virtual address to a physical address.
    ///
    /// The algorithm for virtual-to-physical address translation is described in
    /// RISC-V Privileged Spec Section 4.3.2.
    ///
    /// If a page fault would occurs, this returns `None`; otherwise it returns
    /// `Some` with the physical address.
    fn virt_to_phys(&self, v_addr: usize) -> Option<usize>;

    /// Walk the page table and free the *branch entry* that refers to a sub-table
    /// without any `Valid` entry.
    ///
    /// Returns `true` if the PTE was changed (if any entry was released), otherwise
    /// returns `false`.
    fn free_unused_entry(&mut self) -> bool;

    /// Destroy the entire page table, frees the memory associated with the table.
    ///
    /// **Note**: This method will free `self` too, so the reference of `self` will
    /// be invalid after this call.
    unsafe fn destroy(&mut self);
}

fn cast_to_table<T: Table + 'static>() -> *mut dyn Table {
    let page = allocator::alloc_zeroed_page();
    if page == 0 {
        null_mut::<T>() as *mut dyn Table
    } else {
        page as *mut T as *mut dyn Table
    }
}

/// Create a root table with the special `mode`. Return a trait object pointer that
/// holds all implementations for the input mode.
///
/// **Call Convention**: This function **must** be called from the M-mode or in the
/// S-mode with suitable identity PTEs are set.
pub fn create_root_table(mode: Mode) -> *mut dyn Table {
    match mode {
        Mode::Bare => {
            &mut BareTable as *mut dyn Table
        }
        Mode::Sv39 => {
            cast_to_table::<Sv39Table>()
        }
        Mode::Sv48 => {
            cast_to_table::<Sv48Table>()
        }
        Mode::Sv57 => {
            cast_to_table::<Sv57Table>()
        }
    }
}

/// Copy the root table content. If any entry of the root table is a branch, the
/// sub-table will not be copied, and the new table will refer to the same sub-level
/// tables.
///
/// **Call Convention**: This function **must** be called from the M-mode or in the
/// S-mode with suitable identity PTEs are set.
pub fn copy_root_table(root: &dyn Table) -> *mut dyn Table {
    let pt_addr = allocator::alloc_page();
    let addr = root.get_addr();
    // Page table for each modes has the same size that equals to `PAGE_SIZE`,
    // just copy with ignoring the underlying format.
    let pt_ptr = pt_addr as *mut u64;
    let ptr = addr as *const u64;
    for i in 0..PAGE_SIZE / 8 {
        // Force a ld and sd instruction.
        unsafe {
            *pt_ptr.add(i) = *ptr.add(i);
        }
    }

    unsafe {
        build_table_from_addr(pt_addr, root.get_mode())
    }
}

/// Build a `Table` trait object pointer from the page table physical address and
/// the corresponding `Mode`.
///
/// **Call Convention**: This function **must** be called from the M-mode or in the
/// S-mode with suitable identity PTEs are set.
///
/// # Safety
///
/// The caller **must** make sure that the `mode` is correctly matching the page
/// table address.
pub unsafe fn build_table_from_addr(addr: usize, mode: Mode) -> *mut dyn Table {
    match mode {
        Mode::Bare => {
            &mut BareTable as *mut dyn Table
        }
        Mode::Sv39 => {
            addr as *mut Sv39Table as *mut dyn Table
        }
        Mode::Sv48 => {
            addr as *mut Sv48Table as *mut dyn Table
        }
        Mode::Sv57 => {
            addr as *mut Sv57Table as *mut dyn Table
        }
    }
}


//////////// IMPL OF TABLE TRAIT ///////////////

// Mask to read each level of the PPN and VPN.
const L_MASK: usize = 0x1ff;
// Mask to read PTE flags and PPN value.
const PTE_FLAG_MASK: u64 = 0x3ff;
const PTE_PPN_MASK: u64 = !0x3ff;

const PTE_SIZE: usize = 8;

/// Common map function for Sv39, Sv48, Sv57 mode.
fn do_map<const LEVELS: usize>(
    root: usize,
    v_addr: usize, p_addr: usize,
    bits: u32, level: u32) {
    debug_assert!(level < LEVELS as u32);
    // The virtual address and physical address should align to the corresponding
    // page size.
    debug_assert!(v_addr & ((1usize << (level * 9 + PAGE_ORDER as u32)) - 1) == 0);
    debug_assert!(p_addr & ((1usize << (level * 9 + PAGE_ORDER as u32)) - 1) == 0);

    // Make sure the RWX bits are set and valid.
    assert!(is_bits_valid_leaf(bits));

    // Top PPN value need special mask.
    let ppn_mask = (1usize << (44usize - (LEVELS - 1) * 9)) - 1;

    // On Sv39, Sv48, Sv57 modes, each VPN is exactly 9 bits; PPN[LEVELS-2:0] is also
    // exactly 9 bits while PPN[LEVELS-1] is (44-(LEVELS-1)*9) bits.
    // We init the `ppn` with 0 to satisfy the align requirement when `level` is not 0.
    let mut ppn = [0usize; LEVELS];
    // Read top level VPN.
    let vpn = (v_addr >> ((LEVELS - 1) * 9 + PAGE_ORDER)) & L_MASK;
    ppn[LEVELS - 1] = (p_addr >> ((LEVELS - 1) * 9 + PAGE_ORDER)) & ppn_mask;

    // Read the first PTE.
    let v = (root + vpn * PTE_SIZE) as *mut Entry;
    let mut v = unsafe { &mut *v };
    // Traverse the page table.
    for i in (level as usize..LEVELS - 1).rev() {
        if v.is_invalid() {
            // Alloc a page.
            let page = allocator::alloc_zeroed_page();
            // A page is already aligned by 4096 bytes, so store it in the
            // entry by right shift 2 bits (12 -> 10).
            v.set_entry((page as u64 >> 2) | EntryBits::Valid.val_u64());
        }
        // Entry 'v' must be a branch.
        debug_assert!(v.is_branch());
        let entry = ((v.get_entry() & PTE_PPN_MASK) << 2) as *mut Entry;
        let vpn = (v_addr >> (i * 9 + PAGE_ORDER)) & L_MASK;
        v = unsafe { &mut *entry.add(vpn) };

        ppn[i] = (p_addr >> (i * 9 + PAGE_ORDER)) & L_MASK;
    }

    // Shift each ppn and combine
    let mut entry = (bits as u64 & PTE_FLAG_MASK) | EntryBits::Valid.val_u64();
    for (i, p) in ppn.iter().enumerate() {
        entry |= (p << (i * 9 + 10)) as u64;
    }
    v.set_entry(entry);
}

/// Common unmap function for Sv39, Sv48, Sv57 modes.
/// If the PTE is changed, return true, otherwise return false.
fn do_unmap<const LEVELS: usize>(root: usize, v_addr: usize) -> bool {
    let mut entry = root as *mut Entry;

    for i in (0..LEVELS).rev() {
        let vpn = (v_addr >> (i * 9 + PAGE_ORDER)) & L_MASK;
        let v = unsafe { &mut *entry.add(vpn) };
        if v.is_invalid() {
            debug_assert!(false, "Unmap an invalid address.");
            return false;
        }
        if v.is_leaf() {
            // Find the entry, clear and mark as invalid.
            v.set_entry(0);
            // We will later free the unused page table.
            return true;
        }
        // Entry is a branch.
        entry = ((v.get_entry() & PTE_PPN_MASK) << 2) as *mut Entry;
    }

    // We should not run to here because the page table level is limit.
    debug_assert!(false, "Invalid page table.");
    false
}

/// Common software implementation of the address-translation algorithm with
/// `PTESIZE == 8`.
///
/// The algorithm for virtual-to-physical address translation is described in
/// RISC-V Privileged Spec Section 4.3.2.
fn do_virt2phys<const LEVELS: usize>(root: usize, v_addr: usize) -> Option<usize> {
    let mut entry = root as *mut Entry;

    for i in (0..LEVELS).rev() {
        let shift = i * 9 + PAGE_ORDER;
        let vpn = (v_addr >> shift) & L_MASK;
        let v = unsafe { &mut *entry.add(vpn) };
        // We here only check the `Valid` bit, other flag bits should be checked
        // when do map operation.
        if v.is_invalid() {
            break;
        }
        if v.is_leaf() {
            // If the page is not in the physical memory (for example, swapped to
            // the disk), then the `Access` bit is clear, and the entry's ppn is
            // the ID of disk content.
            if v.is_access_clear() {
                break;
            }

            // Offset mask to read PPN prefix and v-addr offset part.
            let mask = (1usize << shift) - 1usize;
            let va_offset = v_addr & mask;
            let pn = ((v.get_entry() << 2) as usize) & !mask;
            return Some(pn | va_offset);
        }

        // Branch, read next.
        entry = ((v.get_entry() & PTE_PPN_MASK) << 2) as *mut Entry;
    }

    None
}

fn leaf_table_is_used(addr: usize) -> bool {
    let ptr = addr as *const u64;
    let mut valid = 0u64;
    for i in 0..ENTRIES_LEN {
        valid |= unsafe { *ptr.add(i) };
    }

    valid & EntryBits::Valid.val_u64() != 0
}

fn walk_and_free_unused(addr: usize, level: u32, max_level: u32) -> (bool, bool) {
    if level >= max_level {
        return (leaf_table_is_used(addr), false);
    }

    let ptr = addr as *mut Entry;
    let mut valid = 0u64;
    let mut update = false;
    for i in 0..ENTRIES_LEN {
        let v = unsafe { &mut *ptr.add(i) };
        if v.is_invalid() {
            continue;
        }

        if v.is_leaf() {
            valid |= v.get_entry();
        } else {
            let e = ((v.get_entry() & PTE_PPN_MASK) << 2) as usize;
            let (b_v, b_u) = walk_and_free_unused(e, level + 1, max_level);
            if b_v {
                // Sub level table has at least a valid entry.
                valid |= 0x1u64;
                update |= b_u;
            } else {
                // All entries of sub level table have unmapped.
                allocator::free_page(e);
                v.set_entry(0);
                update = true;
                // `valid |= 0x0u64` has no effect.
            }
        }
    }

    (valid & EntryBits::Valid.val_u64() != 0, update)
}

/// Common operation to walk and free the unused *branch* entry. Because all the
/// modes supported by RV64 have the same length PTE, the scan process can be
/// unified.
///
/// If any sub-level table page was free, the *branch* entry will be clear. Returns
/// `true` if any PTE has been changed, otherwise returns `false`.
///
/// **Note**: The root table will not be free.
fn do_free_unused_entry<const LEVELS: usize>(root: usize) -> bool {
    let entry = root as *mut Entry;

    let mut update = false;
    for i in 0..ENTRIES_LEN {
        let v = unsafe { &mut *entry.add(i) };
        if v.is_valid() && v.is_branch() {
            let addr = ((v.get_entry() & PTE_PPN_MASK) << 2) as usize;
            let (valid, u) = walk_and_free_unused(addr, 2, LEVELS as u32);
            if valid {
                update |= u;
            } else {
                // All entries of sub level table have been unmapped.
                allocator::free_page(addr);
                v.set_entry(0);
                update = true;
            }
        }
    }

    update
}

/// Recursive destroy the page table.
///
/// **Note**: Current table (`addr`) will also be destroyed.
fn do_destroy(addr: usize, level: u32, max_level: u32) {
    if level < max_level {
        let entry = addr as *const Entry;

        for i in 0..ENTRIES_LEN {
            let v = unsafe { &*entry.add(i) };
            if v.is_valid() && v.is_branch() {
                let child = (v.get_entry() & PTE_PPN_MASK) << 2;
                do_destroy(child as usize, level + 1, max_level);
            }
        }
    }

    allocator::free_page(addr);
}

const ENTRIES_LEN: usize = 512;

#[repr(C)]
struct Sv39Table {
    entries: [Entry; ENTRIES_LEN],
}

impl Sv39Table {
    const LEVELS: usize = 3;
}

impl Table for Sv39Table {
    fn get_addr(&self) -> usize {
        self as *const Sv39Table as usize
    }

    fn get_mode(&self) -> Mode {
        Mode::Sv39
    }

    fn map(&mut self, v_addr: usize, p_addr: usize, bits: u32, level: u32) {
        do_map::<{ Sv39Table::LEVELS }>(self.get_addr(), v_addr, p_addr, bits, level);
    }

    fn unmap(&mut self, v_addr: usize) -> bool {
        do_unmap::<{ Sv39Table::LEVELS }>(self.get_addr(), v_addr)
    }

    fn virt_to_phys(&self, v_addr: usize) -> Option<usize> {
        do_virt2phys::<{ Sv39Table::LEVELS }>(self.get_addr(), v_addr)
    }

    fn free_unused_entry(&mut self) -> bool {
        do_free_unused_entry::<{ Sv39Table::LEVELS }>(self.get_addr())
    }

    unsafe fn destroy(&mut self) {
        do_destroy(self.get_addr(), 1, Sv39Table::LEVELS as u32);
    }
}

#[repr(C)]
struct Sv48Table {
    entries: [Entry; ENTRIES_LEN],
}

impl Sv48Table {
    const LEVELS: usize = 4;
}

impl Table for Sv48Table {
    fn get_addr(&self) -> usize {
        self as *const Sv48Table as usize
    }

    fn get_mode(&self) -> Mode {
        Mode::Sv48
    }

    fn map(&mut self, v_addr: usize, p_addr: usize, bits: u32, level: u32) {
        do_map::<{ Sv48Table::LEVELS }>(self.get_addr(), v_addr, p_addr, bits, level);
    }

    fn unmap(&mut self, v_addr: usize) -> bool {
        do_unmap::<{ Sv48Table::LEVELS }>(self.get_addr(), v_addr)
    }

    fn virt_to_phys(&self, v_addr: usize) -> Option<usize> {
        do_virt2phys::<{ Sv48Table::LEVELS }>(self.get_addr(), v_addr)
    }

    fn free_unused_entry(&mut self) -> bool {
        do_free_unused_entry::<{ Sv48Table::LEVELS }>(self.get_addr())
    }

    unsafe fn destroy(&mut self) {
        do_destroy(self.get_addr(), 1, Sv48Table::LEVELS as u32);
    }
}

#[repr(C)]
struct Sv57Table {
    entries: [Entry; ENTRIES_LEN],
}

impl Sv57Table {
    const LEVELS: usize = 5;
}

impl Table for Sv57Table {
    fn get_addr(&self) -> usize {
        self as *const Sv57Table as usize
    }

    fn get_mode(&self) -> Mode {
        Mode::Sv57
    }

    fn map(&mut self, v_addr: usize, p_addr: usize, bits: u32, level: u32) {
        do_map::<{ Sv57Table::LEVELS }>(self.get_addr(), v_addr, p_addr, bits, level);
    }

    fn unmap(&mut self, v_addr: usize) -> bool {
        do_unmap::<{ Sv57Table::LEVELS }>(self.get_addr(), v_addr)
    }

    fn virt_to_phys(&self, v_addr: usize) -> Option<usize> {
        do_virt2phys::<{ Sv57Table::LEVELS }>(self.get_addr(), v_addr)
    }

    fn free_unused_entry(&mut self) -> bool {
        do_free_unused_entry::<{ Sv57Table::LEVELS }>(self.get_addr())
    }

    unsafe fn destroy(&mut self) {
        do_destroy(self.get_addr(), 1, Sv57Table::LEVELS as u32);
    }
}

/// Mock table handles the **Bare** mode.
struct BareTable;

impl Table for BareTable {
    fn get_addr(&self) -> usize {
        0
    }

    fn get_mode(&self) -> Mode {
        Mode::Bare
    }

    fn map(&mut self, _v_addr: usize, _p_addr: usize, _bits: u32, _level: u32) {}

    fn unmap(&mut self, _v_addr: usize) -> bool {
        false
    }

    fn virt_to_phys(&self, v_addr: usize) -> Option<usize> {
        Some(v_addr)
    }

    fn free_unused_entry(&mut self) -> bool {
        false
    }

    unsafe fn destroy(&mut self) {}
}
