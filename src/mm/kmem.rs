//! Kernel memory management for sub-page level: malloc-like allocation system.

use core::{mem::size_of, ptr::null_mut};
use crate::base::sync::SpinLockPure;
use crate::errno::{E_INVALID, E_NO_SYS};
use crate::mm::page::{gfp::*, alloc_pages, Page, PAGE_ALLOC_COSTLY_ORDER, PAGE_ALLOC_MAX_ORDER, GfpAllocFlag};
use crate::mm::{PAGE_ORDER, PAGE_SIZE};
use crate::smp::{get_cpu_count, PerCpuPtr};
use crate::util::align::{align_up, align_up_by, align_up_of};
use crate::util::forward_list::ForwardList;
use crate::util::list::List;


/// Flags to pass to [`KmemCache::create`]. The ones marked `Debug` are only valid if
/// **slab debug** is enabled.
///
/// [`KmemCache::create`]: crate::mm::kmem::KmemCache::create
pub mod slab_flags {
    /// Align objs on cache line.
    pub const HWCACHE_ALIGN: u32 = 1u32 << 13;
    pub const SLAB_CACHE_DMA: u32 = 1u32 << 14;
    pub const SLAB_CACHE_DMA32: u32 = 1u32 << 15;
    pub const SLAB_RECLAIM_ACCOUNT: u32 = 1u32 << 17;
}


/// The memory layout of a slab object:
///
/// ```
/// +                                              +
/// |------------------+ size +--------------------|
/// +-------+----------+------------+--------------+
/// | void* |          |            |              |
/// +-------+  object  | word align | object align |
/// +------------------+------------+--------------+
/// ```
///
/// All objects of a slab are organized by singly linked-list, the `next` pointer of
/// linked-list is overlapped with the `object` memory.
#[repr(C)]
pub struct KmemCache {
    cpu_slab: PerCpuPtr<KmemCacheCpu>,
    /// The size of an object including meta data (and alignment).
    size: u32,
    /// The object size without meta data.
    object_size: u32,
    /// Object count that a slab contains.
    object_count: u16,
    /// The order used when alloc pages memory from buddy-system.
    page_order: u16,
    /// Number of per cpu partial slabs to keep around.
    cpu_partial_slabs: u32,
    /// Max number of the node partial slabs to keep around.
    node_partial_slabs: u32,
    /// Used for retrieving partial slabs, etc.
    flags: u32,
    /// Flags to use on each page alloc.
    alloc_flags: u32,
    /// Ref count for slab cache destroy.
    ref_count: u32,
    /// Alignment.
    align: u32,
    /// Reserved bytes at the end of slabs.
    reserved_bytes: u32,
    /// Name (only used for display). We do not use `&str` to avoid the **UB** that when we
    /// get the `KmemCache` object with a `core::mem::zeroed` call.
    name: *const u8,
    node: *mut KmemCacheNode,
    list: List,
}

/// Manage the CPU private cache slabs.
#[repr(C)]
struct KmemCacheCpu {
    /// Points to next available object.
    free_list: ForwardList,
    // RISC-V provides the LR/SC atomic instructions, that can be used to implement a CAS
    // operation without the ABA problem. So we do not need this tid field.
    // pub tid: usize,
    /// Points to the slab from which we are allocating.
    page: *mut Page,
    /// Partially allocated frozen slabs.
    partial: *mut Page,
}

/// The slab lists for all objects.
#[repr(C)]
struct KmemCacheNode {
    list_lock: SpinLockPure,
    nr_partial: usize,
    partial: List,
}

//////////////////// kmem_cache impl /////////////////////////

/// State of the slub allocator.
///
/// This is used to describe the states of the allocator during startup. Allocators
/// use this to gradually bootstrap themselves.
#[repr(u32)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
enum SlabState {
    /// No slub functionality yet.
    Down,
    /// SLUB: kmem_cache_node available.
    Partial,
    /// SLUB: kmalloc size for node struct available.
    PartialNode,
    /// Slub caches usable but not all extras yet.
    Up,
    /// Everything is working.
    Full,
}

// static variables used to bootstrap the kmem_cache data struct.
static mut SLAB_STATE: SlabState = SlabState::Down;
// static mut SLAB_MUTEX
/// The list of all slub caches on the system.
static mut SLAB_CACHES: List = List::new();
/// The slub cache that manages `KmemCache` information.
static mut KMEM_CACHE: *mut KmemCache = null_mut();

/// The slub cache that manages `KmemCacheNode` information.
static mut KMEM_CACHE_NODE: *mut KmemCache = null_mut();

impl KmemCache {
    /// Create
    pub fn create(name: &'static str, object_size: u32, flags: u32) -> *mut KmemCache {
        null_mut()
    }

    pub fn destroy(cache: *mut KmemCache) {
    }

    pub fn alloc(&mut self, flags: u32) -> *mut () {
        null_mut()
    }

    pub fn free(&mut self, obj: *mut ()) {
    }
}

// bootstrap slub allocator.

const ARCH_KMALLOC_MIN_ALIGN: u32 = core::mem::align_of::<u64>() as u32;

/// Init the slub allocator.
///
/// > `init` function.
pub(super) fn kmem_cache_init() {
    let mut boot_kmem_cache: KmemCache = unsafe { core::mem::zeroed() };
    let mut boot_kmem_cache_node: KmemCache = unsafe { core::mem::zeroed() };

    unsafe {
        KMEM_CACHE_NODE = &mut boot_kmem_cache_node as _;
        KMEM_CACHE = &mut boot_kmem_cache as _;
    }
}

/// Create a cache during boot when no slab services are available yet.
fn create_boot_cache(s: &mut KmemCache, name: &'static str, size: u32, flags: u32) {
    s.name = name.as_ptr();
    s.object_size = size;
    s.size = size;

    let align = if size.is_power_of_two() {
        core::cmp::max(ARCH_KMALLOC_MIN_ALIGN, size)
    } else {
        ARCH_KMALLOC_MIN_ALIGN
    };
    s.align = calc_alignment(flags, align, size);

    // s.user_offset & user_size
}

/// Figure out what the alignment of the objects will be given a set of `flags`, a user
/// specified alignment and the `size` of the object.
fn calc_alignment(flags: u32, mut align: u32, size: u32) -> u32 {
    if flags & slab_flags::HWCACHE_ALIGN != 0 {
        // todo: read cache line size from cpu_info.
        let mut r_align = 64u32;
        while size <= r_align / 2 {
            r_align = r_align / 2;
        }
        align = core::cmp::max(align, r_align);
    }

    // align = max(align, arch_slab_min_align)
    align_up_of::<*const ()>(align as usize) as u32
}

fn kmem_cache_create(s: &mut KmemCache, flags: u32) -> i32 {
    let err = kmem_cache_open(s, flags);
    if err != 0 {
        return err;
    }

    unsafe {
        // Mutex is not taken during early boot.
        if SLAB_STATE <= SlabState::Up {
            return 0;
        }
    }

    // todo: sysfs add slab cache

    0
}

fn kmem_cache_release(s: &mut KmemCache) {
    //
}

const MAX_PARTIAL: u32 = 10;
const MIN_PARTIAL: u32 = 5;

fn kmem_cache_open(s: &mut KmemCache, flags: u32) -> i32 {
    s.flags = kmem_cache_apply_debug_flags(s.size, flags, s.name);

    let result = loop {
        if !calc_sizes(s) {
            break Err(());
        }

        // The larger the object size is, the more slabs we want on the partial list to
        // avoid pounding the page allocator excessively.
        s.node_partial = core::cmp::min(MAX_PARTIAL, s.size.ilog2() / 2u32);
        s.node_partial = core::cmp::max(MIN_PARTIAL, s.node_partial);
        set_cpu_partial(s);

        let state = unsafe { SLAB_STATE };
        if state >= SlabState::Up {
            // init random seq
        }

        if init_kmem_cache_nodes(s) == 0 {
            break Err(());
        }
        if alloc_kmem_cache_cpus(s) == 0 {
            break Err(());
        }

        break Ok(());
    };

    if result.is_err() {
        kmem_cache_release(s);
        -E_INVALID
    } else {
        0
    }
}

/// Parse and apply the debug flags.
fn kmem_cache_apply_debug_flags(_object_size: u32, flags: u32, _name: *const u8) -> u32 {
    return flags;
}

/// Determines the order and the distribution of data within a slab object.
fn calc_sizes(s: &mut KmemCache) -> bool {
    let flags = s.flags;
    let mut size = s.object_size;

    // Round up object size to the next word boundary. We can only place the free pointer at
    // word boundaries and this determines the possible location of the free pointer.
    size = align_up_of::<*const ()>(size as usize) as u32;
    // SLUB stores one object immediately after another beginning from offset 0. In order to
    // align the objects we have to simply size each object to conform to the alignment.
    size = align_up_by(size as usize, s.align as usize) as u32;
    s.size = size;

    let order = calc_order(size) as u32;
    if (order as i32) < 0 {
        return false;
    }

    s.alloc_flags = 0;
    if order != 0 {
        s.alloc_flags |= GFP_COMPOUND;
    }
    if flags & slab_flags::SLAB_CACHE_DMA != 0 {
        s.alloc_flags |= GFP_DMA;
    }
    if flags & slab_flags::SLAB_CACHE_DMA32 != 0 {
        s.alloc_flags |= GFP_DMA32;
    }
    if flags & slab_flags::SLAB_RECLAIM_ACCOUNT != 0 {
        s.alloc_flags |= GFP_RECLAIMABLE;
    }

    s.page_order = order as u16;
    s.object_count = order_objects(order, size) as u16;

    s.object_count != 0
}

/// Calculates the best order used to alloc pages for a slub with the special `size` object.
/// Returning a value less than 0 means that we cannot find an appropriate order.
fn calc_order(size: u32) -> i32 {
    const SLUB_MAX_ORDER: u32 = PAGE_ALLOC_COSTLY_ORDER;

    let nr_cpus = get_cpu_count();
    let mut min_objects = (32u32 - nr_cpus.leading_zeros()) * 4;
    let max_objects = order_objects(SLUB_MAX_ORDER, size);
    min_objects = core::cmp::min(min_objects, max_objects);

    while min_objects > 1 {
        let mut fraction = 16u32;
        while fraction >= 4u32 {
            let order = calc_slab_order(size, min_objects, SLUB_MAX_ORDER, fraction);
            if order <= SLUB_MAX_ORDER {
                return order as _;
            }
            fraction /= 2u32;
        }
        min_objects -= 1;
    }

    // We were unable to place multiple objects in a slab. Now lets see if we can place
    // a single object there.
    let order = calc_slab_order(size, 1, SLUB_MAX_ORDER, 1);
    if order <= SLUB_MAX_ORDER {
        return order as _;
    }

    // This slab cannot be placed using SLUB_MAX_ORDER.
    let order = calc_slab_order(size, 1, PAGE_ALLOC_MAX_ORDER, 1);
    if order < PAGE_ALLOC_MAX_ORDER {
        return order as _;
    }

    -E_NO_SYS
}

const MAX_OBJS_PER_PAGE: u32 = 1u32 << 16 - 1u32;

/// Calculates the order of allocation given an slab object size.
///
/// Generally order 0 allocations should be preferred since order 0 does not cause fragmentation
/// in the page allocator. We go to a higher order if more than 1/16th of the slab would be
/// wasted.
///
/// In order to reach satisfactory performance we must ensure that a minimum number of objects
/// is in the slab. Otherwise we may generate too much activity on the partial lists which
/// requires taking the list_lock.
///
/// `max_order` specifies the order where we begin to stop considering the number of objects
/// in a slab as critical. If we reach `max_order` then we try to keep the page order as low
/// as possible. So we accept more waste of space in favor of a small page order.
fn calc_slab_order(size: u32, min_objects: u32, max_order: u32, fract_leftover: u32) -> u32 {
    let min_order = 0u32;
    if order_objects(min_order, size) > MAX_OBJS_PER_PAGE {
        return get_order((size * MAX_OBJS_PER_PAGE) as usize) - 1;
    }

    let mut order = core::cmp::max(min_order, get_order((size * min_objects) as usize));
    while order <= max_order {
        let slab_size = PAGE_SIZE << order;
        let rem = slab_size % size as usize;
        if rem <= (slab_size / fract_leftover as usize) {
            break;
        }

        order += 1;
    }

    order
}

/// Calculates the object count for a slub allocating the `order` page.
#[inline(always)]
const fn order_objects(order: u32, size: u32) -> u32 {
    ((PAGE_SIZE << order) / size) as _
}

/// Determine the allocation order of a memory size. The result is undefined if the `size` is 0.
#[inline(always)]
const fn get_order(mut size: usize) -> u32 {
    size -= 1usize;
    size >>= PAGE_ORDER;
    usize::BITS - size.leading_zeros()
}

fn set_cpu_partial(s: &mut KmemCache) {
    let nr_objects: u32 = if !kmem_cache_has_cpu_partial(s) {
        0
    } else if s.size >= PAGE_SIZE as u32 {
        6
    } else if s.size >= 1024 {
        24
    } else if s.size >= 256 {
        52
    } else {
        120
    };

    slub_set_cpu_partial(s, nr_objects);
}

#[inline]
const fn kmem_cache_has_cpu_partial(_s: &KmemCache) -> bool {
    true
}

#[inline]
fn slub_set_cpu_partial(s: &mut KmemCache, nr_objects: u32) {
    // We take the number of objects but actually limit the number of slabs on the per cpu
    // partial list, in order to limit excessive growth of the list. For simplicity we assume
    // that the slabs will be half-full.
    // todo: add math::div_round_up!(n, d) instead of the expr.
    let nr_slabs = (nr_objects * 2 + s.object_count as u32 - 1) / s.object_count as u32;
    s.cpu_partial_slabs = nr_slabs;
}

fn init_kmem_cache_nodes(s: &mut KmemCache) -> bool {
    // Currently we only support one slab node.
    unsafe {
        if SLAB_STATE == SlabState::Down {
            early_kmem_cache_node_alloc();
            return true;
        }

        let n = kmem_cache_alloc_node(&mut *KMEM_CACHE_NODE, GFP_KERNEL);
        if n.is_null() {
            free_kmem_cache_nodes(s);
            return false;
        }

        let n = n as *mut KmemCacheNode;
        init_kmem_cache_node(n);
        s.node = n;
    }

    true
}

/// No kmalloc_node yet so do it by hand. This is the first slab on the node for this slab cache.
/// There is no concurrent accesses possible.
///
/// Note: this function only works on the `KMEM_CACHE_NODE` when allocating for the `KmemCacheNode`
/// object. This is used for bootstrapping memory on a fresh node that has no slab structures yet.
unsafe fn early_kmem_cache_node_alloc() {
    assert!((*KMEM_CACHE_NODE).size as usize >= size_of::<KmemCacheNode>());
    //
}

fn alloc_slab_page(flags: GfpAllocFlag, order: u32) -> *mut () {
    //
}

fn kmem_cache_alloc_node(s: &mut KmemCache, gfp_flags: GfpAllocFlag) -> *mut () {

}

fn free_kmem_cache_nodes(s: &mut KmemCache) {

}

fn init_kmem_cache_node(n: *mut KmemCacheNode) {

}

fn alloc_kmem_cache_cpus(s: &mut KmemCache) -> i32 {
    //
}


#[repr(usize)]
enum AllocListFlags {
    Taken = 1 << 63,
}

impl AllocListFlags {
    #[inline]
    pub const fn val(self) -> usize {
        self as usize
    }
}

struct AllocList {
    flags_size: usize,
}

impl AllocList {
    #[inline]
    pub const fn is_taken(&self) -> bool {
        self.flags_size & AllocListFlags::Taken.val() != 0
    }

    #[inline]
    pub const fn is_free(&self) -> bool {
        !self.is_taken()
    }

    #[inline]
    pub fn set_taken(&mut self) {
        self.flags_size |= AllocListFlags::Taken.val();
    }

    #[inline]
    pub fn set_free(&mut self) {
        self.flags_size &= !AllocListFlags::Taken.val();
    }

    #[inline]
    pub fn set_size(&mut self, s: usize) {
        let flag = self.flags_size & AllocListFlags::Taken.val();
        self.flags_size = flag | (s & !AllocListFlags::Taken.val());
    }

    #[inline]
    pub const fn get_size(&self) -> usize {
        self.flags_size & !AllocListFlags::Taken.val()
    }
}

// This is the head of the allocation.
static mut KMEM_HEAD: *mut AllocList = null_mut();
// Track the memory length (count as page).
static mut KMEM_ALLOC: usize = 0;

// Safe helpers around an unsafe operation of reading static variable.
pub fn get_head() -> *mut u8 {
    unsafe { KMEM_HEAD as *mut u8 }
}

pub fn get_alloc_page_num() -> usize {
    unsafe { KMEM_ALLOC }
}

/// Initialize the kernel's memory.
pub(super) fn kmem_init() {
    unsafe {
        // Allocate 512 kernel pages (512 * 4KiB = 2MiB)
        const ALLOC_COUNT: usize = 512;
        let k_alloc = alloc_pages(0,ALLOC_COUNT.trailing_zeros() as usize);
        debug_assert!(k_alloc != 0);
        let k_alloc = k_alloc as *mut AllocList;
        (*k_alloc).set_free();
        (*k_alloc).set_size(ALLOC_COUNT * PAGE_SIZE);

        KMEM_ALLOC = ALLOC_COUNT;
        KMEM_HEAD = k_alloc;
    }
}

// todo: return *mut ();
/// Allocate sub-page level allocation based on bytes.
///
/// If the function successfully allocates a memory, the memory is guaranteed to be aligned
/// to 8 bytes.
pub fn kmalloc(sz: usize, _flags: usize) -> *mut u8 {
    if sz == 0 {
        return null_mut();
    }

    unsafe {
        let size = align_up(sz, 3) + size_of::<AllocList>();
        let mut head = KMEM_HEAD;
        let tail = (head as *mut u8).add(KMEM_ALLOC * PAGE_SIZE) as *mut AllocList;

        while head < tail {
            let chunk_size = (*head).get_size();
            if (*head).is_free() && size <= chunk_size {
                let rem = chunk_size - size;
                (*head).set_taken();
                if rem > size_of::<AllocList>() {
                    let next = (head as *mut u8).add(size) as *mut AllocList;
                    // There is space remaining here.
                    (*next).set_free();
                    (*next).set_size(rem);
                    (*head).set_size(size);
                } else {
                    // Taking the entire chunk because the remaining space is not enough to save an
                    // `AllocList` structure.
                    (*head).set_size(chunk_size);
                }

                return head.add(1) as *mut u8;
            } else {
                // Move to next list node.
                head = (head as *mut u8).add(chunk_size) as *mut AllocList;
            }
        }
    }

    null_mut()
}

/// Allocate sub-page level allocation based on bytes and zero the memory
pub fn kzalloc(sz: usize, flags: usize) -> *mut u8 {
    let size = align_up(sz, 3);
    let ret = kmalloc(size, flags);

    if !ret.is_null() {
        // We have aligned the size with `1 << 3`, and the return pointer is guaranteed
        // to be aligned to 8 bytes, so we can use the 'big_ptr' to force a sd instruction.
        let bit_ptr = ret as *mut u64;
        for i in 0..(size / 8) {
            unsafe {
                (*bit_ptr.add(i)) = 0;
            }
        }
    }

    ret
}

/// Free a sub-page level allocation
pub fn kfree(ptr: *mut u8) {
    unsafe {
        if !ptr.is_null() {
            let p = (ptr as *mut AllocList).offset(-1);
            if (*p).is_taken() {
                (*p).set_free();
                // After free, see if we can combine adjacent free spots to reduce fragment.
                coalesce();
            }
        }
    }
}

/// Merge smaller chunks into a bigger chunk
fn coalesce() {
    unsafe {
        let mut head = KMEM_HEAD;
        let tail = (head as *mut u8).add(KMEM_ALLOC * PAGE_SIZE) as *mut AllocList;

        while head < tail {
            let size = (*head).get_size();
            let next = (head as *mut u8).add(size) as *mut AllocList;
            if size == 0 {
                // Something broken, heap is bad.
                debug_assert!(false, "AllocList with size == 0");
                break;
            }
            if next >= tail {
                break;
            }
            if (*head).is_free() && (*next).is_free() {
                // Combine two free block
                (*head).set_size(size + (*next).get_size());
                // Then we continue find from the 'head' with new size.
                continue;
            }
            // Current or next is not freed, move to next
            head = next;
        }
    }
}

/// For debugging purposes, print the kmem table
pub fn print_table() {
    unsafe {
        let mut head = KMEM_HEAD;
        let tail = (head as *mut u8).add(KMEM_ALLOC * PAGE_SIZE) as *mut AllocList;
        while head < tail {
            let size = (*head).get_size();
            println_k!("{:p}: Length = {:<10} Taken = {}", head, size, (*head).is_taken());
            head = (head as *mut u8).add(size) as *mut AllocList;
        }
    }
}
