//! MMU operations. Support Sv39, Sv48, and Sv57 mode. Sv32 mode is used in RV32
//! but the main structure and operations is similar.

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
    pub const fn val(self) -> u32 {
        self as u32
    }

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
const fn is_bits_valid(bits: u32) -> bool {
    bits & 0b0110u32 != 0b0100u32
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
    pub fn is_valid(&self) -> bool {
        self.entry & EntryBits::Valid.val_u64() != 0
    }

    /// Check if the **Valid** bit is clear.
    pub fn is_invalid(&self) -> bool {
        !self.is_valid()
    }

    /// Check if this entry is a leaf. A leaf has one or more RWX bits set.
    pub fn is_leaf(&self) -> bool {
        self.entry & EntryBits::ReadWriteExecute.val_u64() != 0
    }

    /// Check if this entry is a branch. A branch is that RWX bits are all zero.
    pub fn is_branch(&self) -> bool {
        !self.is_leaf()
    }

    pub fn set_entry(&mut self, entry: u64) {
        self.entry = entry;
    }

    pub fn get_entry(&self) -> u64 {
        self.entry
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum Mode {
    Bare = 0,
    Sv39 = 8,
    Sv48 = 9,
    Sv57 = 10,
}

impl Mode {
    pub const fn val(self) -> u8 {
        self as u8
    }

    /// Convenience function to make the **MODE** representation in the `satp`
    /// register. The mode value has been left shift to the bits \[63:60].
    pub const fn val_satp(self) -> u64 {
        (self.val() as u64) << 60
    }
}

/// Operations of the page table.
pub trait Table {
    //fn transfer from and to 'Table' ref.
}








