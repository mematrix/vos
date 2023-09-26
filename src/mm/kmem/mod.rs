//! Kernel memory management for sub-page level: malloc-like allocation system.
//!
//! The allocator APIs **must** be called within a task context (in other words, the `sscratch`
//! register **must** contain a valid [`TaskTrapFrame`] pointer which is a part of the
//! [`TaskInfo`] object).
//!
//! [`TaskTrapFrame`]: crate::proc::task::TaskTrapFrame
//! [`TaskInfo`]: crate::proc::task::TaskInfo

mod slub;

use core::{mem::size_of, ptr::null_mut};
use core::ptr::addr_of_mut;
use core::sync::atomic::Ordering;
use crate::arch::atomic::compare_exchange_usize;
use crate::arch::cpu;
use crate::barrier;
use crate::base::irq;
use crate::base::sync::lock;
use crate::errno::{E_INVALID, E_NO_SYS};
use crate::mm::page::{
    self, gfp::*, alloc_pages, Page,
    PAGE_ALLOC_COSTLY_ORDER, PAGE_ALLOC_MAX_ORDER, GfpAllocFlag
};
use crate::mm::{PAGE_ORDER, PAGE_SIZE};
use crate::mm::kmem::slub::Slub;
use crate::sched::PreemptGuard;
use crate::smp::{get_cpu_count, PerCpuPtr};
use crate::util::align::{align_up, align_up_by, align_up_of};
use crate::util::forward_list::ForwardList;
use crate::util::list::{self, List};


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
    free_list: usize,
    // RISC-V provides the LR/SC atomic instructions, that can be used to implement a CAS
    // operation without the ABA problem. So we do not need this tid field.
    // pub tid: usize,
    /// Points to the slab from which we are allocating.
    page: *mut Slub,
    /// Partially allocated frozen slabs.
    partial: *mut Slub,
}

/// The slab lists for all objects.
#[repr(C)]
struct KmemCacheNode {
    partial: List,
    list_lock: lock::SpinLockPure,
    nr_partial: u32,
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

    let kmem_cache_node = unsafe { &mut *KMEM_CACHE_NODE };
    let slab = alloc_slab(kmem_cache_node, GFP_NO_WAIT);
    assert!(!slab.is_null());

    let slab = &mut *slab;
    let n = slab.get_free_list();
    assert_ne!(n, 0);
    let objects = slab.get_objects() - 1u16;
    slab.set_counters(slub::make_counters(objects, slub::get_free_pointer(n), slab.get_frozen()));

    let n = n as *mut KmemCacheNode;
    kmem_cache_node.node = n;
    init_kmem_cache_node(n);

    // No locks need to be taken here as it has just been initialized and there is
    // no concurrent access.
    add_partial_no_lock(&mut *n, slab, false);
}

fn alloc_slab(s: &mut KmemCache, flags: GfpAllocFlag) -> *mut Slub {
    let alloc_gfp = flags | s.alloc_flags;
    let slab = alloc_slab_page(alloc_gfp, s.page_order as u32);
    // #[unlikely]
    if slab.is_null() {
        return null_mut();
    }

    let start = page::page_to_address(Page::from_private(slab));
    let slab = unsafe {
        // SAFETY: page ptr is guaranteed to be aligned.
        &mut *slab
    };
    // Set free list:
    // This time no other thread will access the same page memory, so we use the non-atomic
    // type directly.
    slab.set_counters(slub::make_counters(s.object_count, start, false));
    slab.set_cache(s);

    let mut p = start;
    for _ in 0..s.object_count {
        let next = p + s.size as usize;
        slub::set_free_pointer(p, next);
        p = next;
    }
    slub::set_free_pointer(p, 0);

    slab
}

fn alloc_slab_page(flags: GfpAllocFlag, order: u32) -> *mut Slub {
    let page = page::get_free_pages(flags, order as usize);
    // todo: page set 'slab' bit flag.
    unsafe {
        (&mut *page).cast_private()
    }
}

/// Add slab to node partially allocated list. If `add_to_tail` is `true`, the `slab` will be
/// appended to the tail of list, otherwise be inserted at list head.
fn add_partial_no_lock(n: &mut KmemCacheNode, slab: &mut Slub, add_to_tail: bool) {
    n.nr_partial += 1;
    if add_to_tail {
        list::tail_append(&mut n.partial, slab.get_slab_list());
    } else {
        list::head_append(&mut n.partial, slab.get_slab_list());
    }
}

fn kmem_cache_alloc_node(s: &mut KmemCache, gfp_flags: GfpAllocFlag) -> *mut () {

}

/// Inlined fast-path so that allocation functions (kmalloc, kmem_cache_alloc) have the fast-path
/// folded into their functions. So no function call overhead for requests that can be satisfied
/// on the fast-path.
///
/// The fast-path works by first checking if the lockless `free_list` can be used. If not then
/// [`slab_alloc_preempt_guard`] is called for slow processing. Otherwise we can simple pick the
/// next object from the lockless `free_list`.
///
/// [`slab_alloc_preempt_guard`]: slab_alloc_preempt_guard
#[inline(always)]
fn slab_alloc_node(s: &mut KmemCache, gfp_flags: GfpAllocFlag, orig_size: u32) -> *mut () {
    // Must read kmem_cache cpu data via this cpu ptr. Preemption is enabled. We may switch
    // back and forth between cpus while reading from one cpu area. That does not matter as
    // long as we end up on the original cpu again when doing the cmpxchg.
    //
    // We must guarantee that `free_list` of kmem_cache_cpu has not been changed by other
    // thread (the **ABA** problem), this can be done by the RISC-V LR/SC instructions.
    let object = loop {
        let c = s.cpu_slab.get_raw();
        barrier!();
        unsafe {
            let object = (*c).free_list;
            let slab = (*c).page;
            // [unlikely]
            if object.is_empty() || slab.is_null() {
                break slab_alloc_preempt_guard(s, gfp_flags, orig_size);
            } else {
                let next_object = slub::get_free_pointer(object);
                barrier!();
                // Read this cpu ptr again. Note that we may switch to another cpu again after
                // the reading. But that does not matter because the following cmpxchg call
                // will verify that the freelist have not been changed. If cmpxchg succeeds,
                // the allocated object may be associated with a slab that belongs to another
                // CpuCache because of preemption, but this is safe.
                let cur_cpu = s.cpu_slab.get_raw();
                let cpu_fp = addr_of_mut!((*cur_cpu).free_list);
                if !compare_exchange_usize(cpu_fp, object, next_object) {
                    continue;
                }
                // todo: prefetch free pointer.
                break object as _;
            }
        }
    };

    object
}

/// A wrapper for `slab_alloc` for contexts where preemption is not yet disabled.
fn slab_alloc_preempt_guard(s: &mut KmemCache, gfp_flags: GfpAllocFlag, orig_size: u32)
    -> *mut() {
    let p;
    {
        let mut c = s.cpu_slab.get_ref_mut();
        p = slab_alloc(s, gfp_flags, &mut c, orig_size);
    }

    p
}

/// Slow path. The lockless freelist is empty.
///
/// Processing is still very fast if new objects have been freed to the regular freelist. In that
/// case we simply take over the regular freelist as the lockless freelist and zap the regular
/// freelist.
///
/// If that is not working then we fall back to the partial lists. We take the first element of
/// the freelist as the object to allocate now and move the rest of the freelist to the lockless
/// freelist.
///
/// And if we were unable to get a new slab from the partial slab list then we need to allocate a
/// new slab. This is the slowest path since it involves a call to the page allocator and the setup
/// of a new slab.
///
/// This function is to use when we know that preemption is already disabled.
fn slab_alloc(s: &mut KmemCache, gfp_flags: GfpAllocFlag, c: &mut PreemptGuard<&mut KmemCacheCpu>, orig_size: u32)
    -> *mut() {
    let mut flags: usize = 0;
    let mut freelist: usize = 0;

    'reread_slab: loop {
        let mut slab = read_once!(c.page);
        'redo: loop {
            'pre_new_slab: loop {
                if slab.is_null() {
                    break;  // goto new_slab
                }

                // redo label
                flags = cpu::sstatus_cli_save();
                if slab != c.page {
                    cpu::sstatus_write(flags);
                    continue 'reread_slab;
                }
                freelist = c.free_list;
                if freelist != 0 {
                    break 'reread_slab;
                }
                freelist = get_freelist(slab);
                if freelist == 0 {
                    c.page = null_mut();
                    cpu::sstatus_write(flags);
                    break 'pre_new_slab;
                }

                break 'reread_slab;
            }

            // new_slab label
            loop {
                if c.partial.is_null() {
                    break 'redo;
                }

                flags = cpu::sstatus_cli_save();
                // [[unlikely]]
                if !c.page.is_null() {
                    cpu::sstatus_write(flags);
                    continue 'reread_slab;
                }
                if c.partial.is_null() {
                    cpu::sstatus_write(flags);
                    // we were preempted and partial list got empty.
                    break 'redo;
                }

                slab = c.partial;
                c.page = slab;
                c.partial = slab.get_partial_next();
                cpu::sstatus_write(flags);
                continue 'redo;
            }
        }

        // new_objects label
        loop {
            (freelist, slab) = get_partial_node(s, s.node);
            if freelist != 0 {
                break;
            }

            c.yield_and_run(|| slab = alloc_slab(s, gfp_flags));
            c.update(|| s.cpu_slab.get_ref_mut_raw());
            // unlikely
            if slab.is_null() {
                // error! slab out of memory
                return null_mut();
            }

            // No other reference to the slab yet so we can muck around with it freely without
            // cmpxchg.
            freelist = slab.get_free_list();
            slab.set_counters_part(s.object_count, 0, true);
            // debug: inc slabs count in node struct.
            break;
        }

        // check new slab --> ignore the impl currently.
        // retry_load_slab label
        loop {
            flags = cpu::sstatus_cli_save();
            // unlikely
            if !c.page.is_null() {
                let flush_freelist = c.free_list;
                let flush_slab = c.page;
                c.page = null_mut();
                c.free_list = 0;
                cpu::sstatus_write(flags);
                // deactivate

                continue;
            }

            c.page = slab;
            // goto load_freelist
            break 'reread_slab;
        }
    }

    // load_freelist label
    // `freelist` is pointing to the list of objects to be used. `page` is pointing to the slab
    // from which the objects are obtained. That slab must be frozen for per cpu allocations to
    // work.
    assert!(c.page.get_frozen());
    c.free_list = slub::get_free_pointer(freelist);
    cpu::sstatus_write(flags);
    freelist as _
}

fn get_freelist(slab: *mut Slub) -> usize {
    let slab = unsafe { &mut *slab };
    let free_info = unsafe { &slab.free_list.free };
    let mut counter = free_info.load(Ordering::Acquire);
    loop {
        let freelist = slub::counters_get_free_list(counter);
        let new_counter = slub::make_counters(slub::counters_get_objects(counter), 0, freelist != 0);
        match free_info.compare_exchange_weak(counter, new_counter, Ordering::AcqRel, Ordering::Acquire) {
            Ok(_) => break freelist,
            Err(x) => counter = x,
        }
    }
}

/// Try to allocate a partial slab from a special node and lock it. Returns a list of objects
/// (may be null) and the slab.
fn get_partial_node(s: &mut KmemCache, n: *mut KmemCacheNode) -> (usize, *mut Slub) {
    unsafe {
        if n.is_null() || (*n).nr_partial == 0 {
            return (0, null_mut());
        }
    }

    let n = unsafe { &mut *n };
    let mut object = 0usize;
    let mut slab = null_mut();
    let mut partial_slabs = 0u32;

    let _guard = n.list_lock.lock_guard_irq_save();
    // for each partial list
    list::for_each(&mut n.partial, |cur| {
        let cur_slab = crate::container_of_mut!(cur, Slub, list);
        let ref_slab = unsafe { &mut *cur_slab };
        let t = acquire_slab(s, n, ref_slab, object == 0);
        if t == 0 {
            return false;
        }

        if object == 0 {
            slab = cur_slab;
            object = t;
        } else {
            put_cpu_partial(s, ref_slab, false);
            partial_slabs += 1;
        }

        // if partial_slabs not greater than limit, continue iterating.
        partial_slabs <= s.cpu_partial_slabs / 2
    });

    (object, slab)
}

fn acquire_slab(s: &mut KmemCache, n: &mut KmemCacheNode, slab: &mut Slub, mode: bool) -> usize {
    debug_assert!(n.list_lock.is_locked());
    let counters = slab.get_counters();
    // make sure slab is not frozen
    assert!(!slub::counters_get_frozen(counters));
    let new_counters = if mode {
        let objects = slub::counters_get_objects(counters);
        slub::make_counters(objects, 0, true)
    } else {
        slub::counters_set_frozen(counters, true)
    };

    let free_info = unsafe { &slab.free_list.free };
    match free_info.compare_exchange(counters, new_counters, Ordering::AcqRel, Ordering::Acquire) {
        Ok(_) => {
            remove_partial(n, slab);
            // warn_on! freelist == 0
            slub::counters_get_free_list(counters)
        },
        Err(_) => 0usize,
    }
}

fn remove_partial(n: &mut KmemCacheNode, slab: &mut Slub) {
    list::delete(unsafe { &mut slab.list.slab_list });
    n.nr_partial -= 1;
}

/// Put a slab that was just frozen (in `acquire_slab`) into a partial slab slot if available.
fn put_cpu_partial(s: &mut KmemCache, slab: &mut Slub, drain: bool) {
    let mut slab_to_unfreeze = null_mut();
    let mut slabs = 0u32;

    let flags = irq::local_irq_save();

    // irq disabled, so we read per-cpu variable without preempt-guard.
    let mut old_slab = s.cpu_slab.get_ref_raw().partial;
    if !old_slab.is_null() {
        let tmp = unsafe { &*old_slab };
        if drain && tmp.get_partial_slabs() >= s.cpu_partial_slabs {
            // Partial array is full. Move the existing set to the node partial list.
            slab_to_unfreeze = old_slab;
            old_slab = null_mut();
        } else {
            slabs = tmp.get_partial_slabs();
        }
    }

    slabs += 1;
    slab.set_partial_slabs(slabs);
    slab.set_partial_next(old_slab);
    s.cpu_slab.get_ref_mut_raw().partial = slab as _;

    irq::local_irq_restore(flags);

    if !slab_to_unfreeze.is_null() {
        unfreeze_partials(s, slab_to_unfreeze);
    }
}

/// unfreeze the partial slab list.
fn unfreeze_partials(s: &mut KmemCache, mut partial_slab: *mut Slub) {
    if partial_slab.is_null() {
        return;
    }

    let mut slab_to_discard = null_mut();
    let n = unsafe { &mut *s.node };
    {
        let _guard = n.list_lock.lock_guard_irq_save();
        while !partial_slab.is_null() {
            let slab = unsafe { &mut *partial_slab };
            partial_slab = slab.get_partial_next();

            let mut old = slab.get_counters();
            let counters = loop {
                assert!(slub::counters_get_frozen(old));
                let new = slub::counters_set_frozen(old, false);
                match slab.get_atomic_counters().compare_exchange_weak(old, new, Ordering::AcqRel, Ordering::Acquire) {
                    Ok(_) => break new,
                    Err(v) => old = v,
                }
            };

            // unlikely
            if slub::counters_get_objects(counters) == 0 && n.nr_partial >= s.node_partial_slabs {
                slab.set_partial_next(slab_to_discard);
                slab_to_discard = slab as _;
            } else {
                add_partial_no_lock(n, slab, true);
            }
        }
    }

    while !slab_to_discard.is_null() {
        let slab = slab_to_discard;
        slab_to_discard = unsafe { &mut *slab }.get_partial_next();
        free_slab(s, slab);
    }
}

fn free_slab(s: &mut KmemCache, slab: *mut Slub) {
    let p = Page::from_private(slab);
    page::return_pages(p, s.page_order as usize);
}

fn deactivate_slab(s: &mut KmemCache, slab: &mut Slub, freelist: usize) {
    enum SlabModes {
        None,
        Partial,
        Free,
        FullNoList,
    }

    let n = unsafe { &mut *s.node };
    let tail = slab.get_free_list() != 0;
    let mut mode = SlabModes::None;

    // Stage one: Count the objects on cpu's freelist as free_delta and remember the last
    // object in freelist_tail for later splicing.
    let mut free_delta = 0u32;
    let mut freelist_tail = 0usize;
    let mut freelist_iter = freelist;
    while freelist_iter != 0 {
        let next_free = slub::get_free_pointer(freelist_iter);
        if is_freelist_corrupted(s, slab, next_free) {
            freelist_iter = 0;
            break;
        }

        freelist_tail = freelist_iter;
        free_delta += 1;
        freelist_iter = next_free;
    }
}

fn is_freelist_corrupted(s: &mut KmemCache, slab: &mut Slub, next_free: usize) -> bool {
    !check_valid_pointer(s, slab, next_free)
}

/// Verify that a pointer has an address that is valid within a slab page.
fn check_valid_pointer(s: &mut KmemCache, slab: &mut Slub, object: usize) -> bool {
    if object == 0 {
        return true;
    }

    let base = slab_address(slab);
    let invalid = (object < base) ||
        (object >= base + s.object_count as usize * s.size as usize) ||
        ((object - base) % s.size != 0);
    !invalid
}

/// Get the base address that is associated with a slab page.
fn slab_address(slab: &mut Slub) -> usize {
    let page = Page::from_private(slab as *mut Slub);
    page::page_to_address(page)
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
