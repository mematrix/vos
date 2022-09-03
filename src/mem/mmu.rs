//! MMU operations. Support Sv39, Sv48, and Sv57 mode. Sv32 mode is used in RV32
//! but the main structure and operations is similar.
//!
//! # Note
//!
//! Any MMU function **must** be called from the M-mode or in the S-mode with an
//! identify PTE is set to cover the PageEntry physical memory. Because read and
//! write operation of the PTE all access the physical memory directly.

use crate::mem::page::{PAGE_ORDER, zalloc};


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
/// S-mode with an identify PTE which covers all physical memory that the page
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

/// Create a root table with the special `mode`. Return a trait object pointer that
/// holds all implementations for the input mode.
///
/// **Call Convention**: This function **must** be called from the M-mode or in the
/// S-mode with suitable identify PTEs are set.
pub fn create_root_table(mode: Mode) -> *mut dyn Table {
    //
}

/// Build a `Table` trait object pointer from the page table physical address and
/// the corresponding `Mode`.
///
/// **Call Convention**: This function **must** be called from the M-mode or in the
/// S-mode with suitable identify PTEs are set.
///
/// # Safety
///
/// The caller **must** make sure that the `mode` is correctly matching the page
/// table address.
pub unsafe fn build_table_from_addr(addr: usize, mode: Mode) -> *mut dyn Table {
    //
}


//////////// IMPL OF TABLE TRAIT ///////////////

// Mask to read each level of the PPN and VPN.
const L_MASK: usize = 0x1ff;
// Mask to read PTE flags and PPN value.
const PTE_FLAG_MASK: u64 = 0x3ff;
const PTE_PPN_MASK: u64 = !0x3ff;

/// Common map function for Sv39, Sv48, Sv57 mode.
fn do_map<const LEVELS: u32, const PTE_SIZE: usize, const AUTO_VALID: bool>(
    root: usize,
    v_addr: usize, p_addr: usize,
    bits: u32, level: u32) {
    debug_assert!(level < LEVELS);
    // The virtual address and physical address should align to the corresponding
    // page size.
    debug_assert!(v_addr & ((1usize << (level * 9 + PAGE_ORDER)) - 1) == 0);
    debug_assert!(p_addr & ((1usize << (level * 9 + PAGE_ORDER)) - 1) == 0);

    // Make sure the RWX bits are set and valid.
    assert!(is_bits_valid_leaf(bits));

    // Top PPN value need special mask.
    const L_PPN_MASK: usize = (1usize << (44usize - (LEVELS - 1) * 9)) - 1;

    // On Sv39, Sv48, Sv57 modes, each VPN is exactly 9 bits; PPN[LEVELS-2:0] is also
    // exactly 9 bits while PPN[LEVELS-1] is (44-(LEVELS-1)*9) bits.
    // We init the `ppn` with 0 to satisfy the align requirement when `level` is not 0.
    let ppn = [0usize; LEVELS];
    // Read top level VPN.
    let vpn = (v_addr >> ((LEVELS - 1) * 9 + PAGE_ORDER)) & L_MASK;
    ppn[LEVELS - 1] = (p_addr >> ((LEVELS - 1) * 9 + PAGE_ORDER)) & L_PPN_MASK;

    // Read the first PTE.
    let v = (root + vpn * PTE_SIZE) as *mut Entry;
    let mut v = unsafe { &mut *v };
    // Traverse the page table.
    for i in (level..LEVELS - 1).rev() {
        if v.is_invalid() {
            // Alloc a page.
            let page = zalloc(1);
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
    let mut entry = (bits as u64 & PTE_FLAG_MASK)
        | (AUTO_VALID as u64 & EntryBits::Valid.val_u64());
    for (i, p) in ppn.iter().enumerate() {
        entry |= p << (i * 9 + 10);
    }
    v.set_entry(entry);
}

/// Common unmap function for Sv39, Sv48, Sv57 modes.
/// If the PTE is changed, return true, otherwise return false.
fn do_unmap<const LEVELS: u32>(root: usize, v_addr: usize) -> bool {
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
fn do_virt2phys<const LEVELS: u32>(root: usize, v_addr: usize) -> Option<usize> {
    //
}

/// Common operation to walk and free the unused *branch* entry. Because all the
/// modes supported by RV64 have the same length PTE, the scan process can be
/// unified.
fn do_free_unused_entry<const LEVELS: u32>(root: usize) -> bool {
    //
}

const ENTRIES_LEN: usize = 512;

#[repr(C)]
struct Sv39Table {
    entries: [Entry; ENTRIES_LEN],
}

impl Sv39Table {
    const LEVELS: u32 = 3;

    pub const fn len() -> usize {
        ENTRIES_LEN
    }
}

impl Table for Sv39Table {
    fn get_addr(&self) -> usize {
        self as *const Sv39Table as usize
    }

    fn get_mode(&self) -> Mode {
        Mode::Sv39
    }

    fn map(&mut self, v_addr: usize, p_addr: usize, bits: u32, level: u32) {
        do_map::<Sv39Table::LEVELS, 8, true>(self.get_addr(), v_addr, p_addr, bits, level);
    }

    fn unmap(&mut self, v_addr: usize) -> bool {
        do_unmap::<Sv39Table::LEVELS>(self.get_addr(), v_addr)
    }

    fn virt_to_phys(&self, v_addr: usize) -> Option<usize> {
        do_virt2phys::<Sv39Table::LEVELS>(self.get_addr(), v_addr)
    }

    fn free_unused_entry(&mut self) -> bool {
        do_free_unused_entry::<Sv39Table::LEVELS>(self.get_addr())
    }

    unsafe fn destroy(&mut self) {
        todo!()
    }
}







