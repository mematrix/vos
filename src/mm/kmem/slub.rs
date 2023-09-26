//! SLUB structures definition.

use core::sync::atomic::AtomicUsize;
use crate::mm::KmemCache;
use crate::util::list::List;


/// Reuses the bits in `struct Page`.
#[repr(C)]
pub struct Slub {
    pub list: SlubHead2Word,
    /// Save the free-list head and number of in-use objects. Memory layout (in C decl):
    ///
    /// ``` C
    /// struct {
    ///   uint64_t objects:16;
    ///   uint64_t free_list:47;
    ///   uint64_t frozen:1;
    /// }
    /// ```
    ///
    /// The object will be always aligned with the word size so we can reuse the least 3 bits
    /// to store some flags.
    pub free_list: SlubFreeList,
    pub slab_cache: *mut KmemCache,
}

#[repr(C)]
pub union SlubHead2Word {
    /// List used in node partial.
    pub slab_list: List,
    // pub rcu_head
    pub partial: SlubCpuPartial,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct SlubCpuPartial {
    pub next: *mut Slub,
    /// Number of slabs left.
    pub slabs: u32,
}

/// Used to access the memory both in atomic and non-atomic context.
#[repr(C)]
pub union SlubFreeList {
    pub free: AtomicUsize,
    pub counters: usize,
}

sa::const_assert!(core::mem::size_of::<Slub>() <= crate::mm::page::Page::get_private_size());

////////////////// Helper methods. ////////////////////////

#[inline(always)]
pub const fn counters_set_objects(counters: usize, objects: u16) -> usize {
    (counters & !(0xffffusize << 48)) | ((objects as usize) << 48)
}

#[inline(always)]
pub const fn counters_get_objects(counters: usize) -> u16 {
    (counters >> 48) as u16
}

#[inline(always)]
pub const fn counters_set_free_list(counters: usize, fp: usize) -> usize {
    make_counters((counters >> 48) as u16, fp, (counters & 0x1usize) != 0)
}

#[inline(always)]
pub const fn counters_get_free_list(counters: usize) -> usize {
    counters & (!(0xffffusize << 48) ^ 0x1usize)
}

#[inline(always)]
pub const fn counters_set_frozen(counters: usize, frozen: bool) -> usize {
    (counters & !0x1usize) | (frozen as usize)
}

#[inline(always)]
pub const fn counters_get_frozen(counters: usize) -> bool {
    (counters & 0x1usize) != 0
}

#[inline(always)]
pub const fn objects_to_counters(objects: u16) -> usize {
    (objects as usize) << 48
}

/// Construct counters value. Caller **must** guard that `ptr` is aligned with at least 2 and
/// the `ptr` value is not greater tran `1usize << 48`.
#[inline(always)]
pub const fn make_counters(objects: u16, ptr: usize, frozen: bool) -> usize {
    ((objects as usize) << 48) | ptr | (frozen as usize)
}

impl Slub {
    #[inline(always)]
    pub fn init(&mut self) {
        // todo: init self to default state.
    }

    #[inline(always)]
    pub fn set_objects(&mut self, objects: u16) {
        self.free_list.counters = counters_set_objects(self.free_list.counters, objects);
    }

    #[inline(always)]
    pub fn get_objects(&self) -> u16 {
        counters_get_objects(self.free_list.counters)
    }

    #[inline(always)]
    pub fn set_free_list(&mut self, fp: usize) {
        self.free_list.counters = counters_set_free_list(self.free_list.counters, fp);
    }

    #[inline(always)]
    pub fn get_free_list(&self) -> usize {
        self.free_list.counters & (!(0xffffusize << 48) ^ 0x1usize)
    }

    #[inline(always)]
    pub fn set_frozen(&mut self, frozen: bool) {
        self.free_list.counters = counters_set_frozen(self.free_list.counters, frozen);
    }

    #[inline(always)]
    pub fn get_frozen(&self) -> bool {
        (self.free_list.counters & 0x1usize) != 0
    }

    /// Replace total value in `counters`. Note the `free` ptr also changed.
    #[inline(always)]
    pub fn set_counters(&mut self, counters: usize) {
        self.free_list.counters = counters;
    }

    /// Set `counters` value by passing each part of the **counters** property.
    #[inline(always)]
    pub fn set_counters_part(&mut self, inuse: u16, fp: usize, frozen: bool) {
        self.free_list.counters = make_counters(inuse, fp, frozen);
    }

    #[inline(always)]
    pub fn get_counters(&self) -> usize {
        self.free_list.counters
    }

    pub fn get_atomic_counters(&self) -> &AtomicUsize {
        &self.free_list.free
    }

    #[inline(always)]
    pub fn set_cache(&mut self, s: &mut KmemCache) {
        self.slab_cache = s as _;
    }

    pub fn get_cache(&self) -> *mut KmemCache {
        self.slab_cache
    }

    #[inline(always)]
    pub fn get_slab_list(&mut self) -> &mut List {
        &mut self.list.slab_list
    }

    #[inline(always)]
    pub fn set_partial_next(&mut self, next: *mut Slub) {
        self.list.partial.next = next;
    }

    #[inline(always)]
    pub fn get_partial_next(&self) -> *mut Slub {
        self.list.partial.next
    }

    #[inline(always)]
    pub fn set_partial_slabs(&mut self, slabs: u32) {
        self.list.partial.slabs = slabs;
    }

    #[inline(always)]
    pub fn get_partial_slabs(&self) -> u32 {
        self.list.partial.slabs
    }
}

/// Set the next free-pointer value of `object`. See the [slub objects layout].
///
/// [slub objects layout]: crate::mm::kmem
#[inline(always)]
pub fn set_free_pointer(object: usize, fp: usize) {
    unsafe {
        *(object as *mut usize) = fp;
    }
}

/// Returns the free-list pointer value recorded at location `object`.
#[inline(always)]
pub fn get_free_pointer(object: usize) -> usize {
    unsafe {
        *(object as *mut usize)
    }
}
