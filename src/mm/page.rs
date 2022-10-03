//! Page-based physical memory allocation.
//!
//! The principal algorithm used is the **Binary Buddy Allocator**, which is also used in the
//! Linux OS and documented in [Chapter 6  Physical Page Allocation].
//!
//! The *physical memory* is managed with **page**, the page size is default set to 4KiB. Each
//! **page** has an associated [`Page`] struct object, for more info, see [`Page`].
//!
//! All address or pointer value returned from the allocation API functions are the **physical
//! address**. The returned type of the allocation API may be either `usize` (physical address,
//! points to the allocated memory) or [`Page`] struct (meta info of the page). Use
//! [`page_address`] to convert a [`Page`] to a **physical address**.
//!
//! ## Allocation API
//!
//! | Allocation API | Return | Description |
//! | -------------- | ------ | ----------- |
//! | get_free_page(flag) | `Page` | Allocate a single page and return a struct page |
//! | get_free_pages(flag, order) | `Page` | Allocate 2^order number of pages and return a struct page |
//! | alloc_page(flag) | `usize` | Allocate a single page and return the address |
//! | alloc_pages(flag, order) | `usize` | Allocate 2^order number of pages and return the address |
//! | alloc_zeroed_page(flag) | `usize` | Allocate a single page, zero it and return the address |
//!
//! ## Free API
//!
//! | Free API | Description |
//! | -------- | ----------- |
//! | return_page(page) | Free a single page |
//! | return_pages(page, order) | Free an order number of pages from the given page |
//! | free_page(addr) | Free a single page from the give address |
//! | free_pages(addr, order) | Free an order number of pages from the given address |
//!
//! ## Calling Convention
//! All functions in this mod **must be** called either on the M-mode or on the S-mode with an identity
//! mapping table is set (`SATP`).
//!
//! > If running on S-mode, the identity mapping table must cover all physical memory address range.
//!
//! [Chapter 6  Physical Page Allocation]: https://www.kernel.org/doc/gorman/html/understand/understand009.html
//! [`Page`]: self::Page
//! [`page_address`]: self::page_address

use core::mem::size_of;
use core::ptr::null_mut;
use core::sync::atomic::{AtomicU32, Ordering};
use crate::util::align::{align_down, align_up, get_order};
use crate::util::list::{self, List};


pub const PAGE_ORDER: usize = 12;
/// Page size.
pub const PAGE_SIZE: usize = 1 << 12;


/// Page flags definition.
#[repr(u32)]
#[derive(Copy, Clone)]
pub enum PageFlag {
    /// Page is used in the SLAB/SLUB allocator.
    Slab = 1 << 0,
    /// Page is shared between multiple processes.
    Shared = 1 << 1,
}

impl PageFlag {
    pub const fn val(self) -> u32 {
        self as u32
    }
}

/// Each **page** is described by the `Page` structure.
///
/// We guarantee that `size_of::<Page>() % 16 == 0` and the `Page` object will be allocated
/// with the align of `size_of::<Page>()` or 16. So the least 4 bits of the pointer to a `Page`
/// object will always guard to 0 and can be safety used, such as save the page alloc order.
///
/// If a **page** is allocated, then the `Page` structure associated with it will be free to
/// use to store private data, except for the last 4 bytes which are used to store the page
/// flags. [`get_private`] method can be used to retrieve the private area start address, and
/// [`get_private_size`] method can be used to get the available size of private area, this is
/// a **const** function so it can be called at the compile-time context to do some static
/// assertions. The private size is guaranteed to at least 28 bytes; the private data address
/// is guaranteed to align with 16.
///
/// The least 24 bits of the inner `flags` field are used by the Page allocator, while the most
/// 8 bits can be used to store custom flags. See methods [`get_custom_flags`], [`set_custom_flags`]
/// and [`replace_custom_flags`] for more info, note that though the param or return type is `u32`,
/// only the least 8 bits is valid and used.
///
/// [`get_private`]: self::Page::get_private
/// [`get_private_size`]: self::Page::get_private_size
/// [`get_custom_flags`]: self::Page::get_custom_flags
/// [`set_custom_flags`]: self::Page::set_custom_flags
/// [`replace_custom_flags`]: self::Page::replace_custom_flags
#[repr(C)]
pub struct Page {
    head: List,
    padding: usize,
    ref_count: AtomicU32,
    /// bits\[7:0] store the idx of zone that self page belongs. bits\[23:8] store inner flags.
    /// bits\[31:24] store custom flags.
    flags: u32,
}

// Assert the size of `Page` equals to multiple times of 32bytes.
sa::const_assert_eq!(size_of::<Page>() % 32, 0);

impl Page {
    /// Get the available size for private usage.
    #[inline(always)]
    pub const fn get_private_size() -> usize {
        size_of::<Page>() - size_of::<u32>()
    }

    /// Get a pointer of the private memory.
    ///
    /// **Note**: data in this memory may be in **uninitialized** state.
    #[inline(always)]
    pub fn get_private(&mut self) -> *mut u8 {
        self as *mut Page as *mut u8
    }

    /// Get the flags. **Note**: custom flags are also returned.
    #[inline(always)]
    pub fn read_flags(&self) -> u32 {
        self.flags >> 8
    }

    /// Check if the `flag` is set.
    #[inline(always)]
    pub fn is_flag_set(&self, flag: PageFlag) -> bool {
        ((self.flags >> 8) & flag.val()) != 0
    }

    /// Set page flag.
    #[inline(always)]
    pub fn set_flag(&mut self, flag: PageFlag) {
        self.flags |= flag.val() << 8;
    }

    /// Clear page flag.
    #[inline(always)]
    pub fn clear_flag(&mut self, flag: PageFlag) {
        self.flags &= !(flag.val() << 8);
    }

    /// Get the custom flags.
    #[inline(always)]
    pub fn read_custom_flags(&self) -> u32 {
        self.flags >> 24
    }

    /// Replace the custom flags. This will overwrite the total custom flags value.
    ///
    /// **Note**: Only the least 16 bits of `flag` is used.
    #[inline(always)]
    pub fn replace_custom_flags(&mut self, flag: u32) {
        self.flags = (self.flags & 0xff_ffffu32) | (flag << 24);
    }

    /// Set the certain custom flags. If a bit of `flag` is 1, **set** the correspond custom flag bit;
    /// otherwise leave the correspond flag bit unchanged.
    ///
    /// **Note**: Only the least 16 bits of `flag` is used.
    #[inline(always)]
    pub fn set_custom_flags(&mut self, flag: u32) {
        self.flags |= flag << 24;
    }

    /// Clear the certain custom flags. If a bit of `flag` is 1, then **clear** the correspond custom
    /// flag bit; otherwise leave the correspond flag bit unchanged.
    ///
    /// **Note**: Only the least 16 bits of `flag` is used.
    #[inline(always)]
    pub fn clear_custom_flags(&mut self, flag: u32) {
        self.flags &= !(flag << 24);
    }

    /// Set zone idx.
    #[inline(always)]
    fn set_zone_idx(&mut self, idx: usize) {
        self.flags = (self.flags & !0xffu32) | (idx & 0xffusize) as u32;
    }

    /// Get zone idx.
    #[inline(always)]
    fn get_zone_idx(&self) -> usize {
        (self.flags & 0xffu32) as usize
    }

    /// Get the ref count. **Note**: only available when page has been set [`PageFlag::Shared`] flag.
    ///
    /// [`PageFlag::Shared`]: self::PageFlag::Shared
    #[inline(always)]
    pub fn ref_count(&self) -> u32 {
        self.ref_count.load(Ordering::Relaxed)
    }

    /// Increase the ref count of the shared page.
    ///
    /// **Note**: it is an **Undefined Behavior** if the page has no [`PageFlag::Shared`] flag set.
    ///
    /// [`PageFlag::Shared`]: self::PageFlag::Shared
    #[inline(always)]
    pub fn increase_ref(&mut self) {
        self.ref_count.fetch_add(1, Ordering::AcqRel);
    }

    /// Decrease the ref count and return current ref count (after the decrease operation).
    ///
    /// **Note**: it is an **Undefined Behavior** if the page has no [`PageFlag::Shared`] flag set.
    ///
    /// [`PageFlag::Shared`]: self::PageFlag::Shared
    #[inline(always)]
    pub fn decrease_ref(&mut self) -> u32 {
        self.ref_count.fetch_sub(1, Ordering::AcqRel) - 1u32
    }
}


#[repr(C)]
struct FreeArea {
    free_list: List,
    bitmap: *mut u8,
}

impl FreeArea {
    pub const fn new() -> Self {
        Self {
            free_list: List::new(),
            bitmap: null_mut(),
        }
    }
}

const MAX_FREE_AREA_ORDER: usize = 10;

#[repr(C)]
struct Zone {
    free_areas: [FreeArea; MAX_FREE_AREA_ORDER],
    free_pages: usize,
    max_pages: usize,
    mem_start: usize,
    mem_size: usize,
}

impl Zone {
    pub const fn new() -> Self {
        const VAL: FreeArea = FreeArea::new();
        Self {
            free_areas: [VAL; MAX_FREE_AREA_ORDER],
            free_pages: 0,
            max_pages: 0,
            mem_start: 0,
            mem_size: 0,
        }
    }

    pub fn init(&mut self) {
        for area in &mut self.free_areas {
            area.free_list.init_empty();
        }
    }
}

const MAX_ZONE_COUNT: usize = 1;
/// Memory zone list.
static mut MEMORY_ZONES: [Zone; MAX_ZONE_COUNT] = [Zone::new(); MAX_ZONE_COUNT];
/// `Page` object array base address.
static mut PAGE_OBJ_BASE: usize = 0;
// We will use ALLOC_START to mark the start of the actual
// memory we can dish out.
static mut ALLOC_START: usize = 0;
// Track the max number than can be allocated.
static mut ALLOC_PAGES: usize = 0;


/// Initialize the buddy allocator system.
///
/// **Note**: After this call, the heap base address **must not** be changed.
pub fn init(mem_regions: &[(usize, usize)]) {
    assert!(!mem_regions.is_empty(), "Memory regions is empty!");
    if mem_regions.len() > 1 {
        warn!("Physical memory address is not continuous.");
    }

    unsafe {
        let zone = &mut MEMORY_ZONES[0];
        zone.init();

        let &(mem_start, mem_size) = mem_regions.get_unchecked(0usize);
        zone.mem_start = mem_start;
        zone.mem_size = mem_size;

        let mem_end = mem_start + mem_size;
        const ALIGNMENT: usize = PAGE_SIZE << (MAX_FREE_AREA_ORDER - 1usize);
        let mem_end = align_down(mem_end, get_order(ALIGNMENT));

        let start = super::HEAP_BASE;
        let alloc_min_addr = align_up(start, get_order(ALIGNMENT));
        assert!(alloc_min_addr >= mem_start && alloc_min_addr < mem_end);
        let max_alloc_pages = (mem_end - alloc_min_addr) / PAGE_SIZE;

        // Init the free area bitmap.
        // We alloc the bitmap with align of 8bytes.
        let mut bitmap_start = align_up(start, get_order(size_of::<u64>()));
        let mut bitmap_len = 0usize;
        for i in 0..(MAX_FREE_AREA_ORDER - 1) {
            bitmap_len += (max_alloc_pages >> (i + 1usize) + 7) / 8;
        }
        let page_start = align_up(bitmap_start + bitmap_len, get_order(32usize));
        // Cast bitmap to u64 pointer and memset to zero.
        let bitmap_ptr = bitmap_start as *mut u64;
        bitmap_ptr.write_bytes(0, (page_start - bitmap_start) / size_of::<u64>());
        // Init
        for i in 0..(MAX_FREE_AREA_ORDER - 1) {
            let free_area = zone.free_areas.get_unchecked_mut(i);
            free_area.bitmap = bitmap_start as *mut u8;

            bitmap_start += ((max_alloc_pages >> (i + 1usize)) + 7) / 8;
        }

        // Adjust the min alloc address
        let max_alloc_large_pages = (mem_end - page_start) /
            ((PAGE_SIZE + size_of::<Page>()) << (MAX_FREE_AREA_ORDER - 1usize));
        let alloc_pages = max_alloc_large_pages << (MAX_FREE_AREA_ORDER - 1usize);
        let page_end = page_start + size_of::<Page>() * alloc_pages;
        let alloc_start = align_up(page_end, get_order(ALIGNMENT));

        // Init `Page` objects.
        let free_area = zone.free_areas.get_unchecked_mut(MAX_FREE_AREA_ORDER - 1usize);
        let list_head = &mut free_area.free_list;
        let mut prev_node = list_head as *mut List;
        let page_base = page_start as *mut Page;
        const PAGE_COUNT_LAST_AREA: usize = 1usize << (MAX_FREE_AREA_ORDER - 1usize);
        for i in 0..max_alloc_large_pages {
            // All `Page`obj to free_area[MAX_ORDER - 1].free_list.
            let page = page_base.add(i * PAGE_COUNT_LAST_AREA);
            // (*page).flags = 0;
            let page_head = &mut (*page).head;
            list::partial_append(&mut *prev_node, page_head);
            prev_node = page_head as _;
        }
        list::partial_append(&mut *prev_node, list_head);

        PAGE_OBJ_BASE = page_start;
        ALLOC_START = alloc_start;
        ALLOC_PAGES = alloc_pages;
        zone.free_pages = alloc_pages;
        zone.max_pages = alloc_pages;
    }
}

/// Allocate a single page and return a struct page.
pub fn get_free_page(flags: usize) -> *mut Page {
    do_alloc_pages(flags, 0)
}

/// Allocate `2^order` number of pages and return a struct page.
pub fn get_free_pages(flags: usize, order: usize) -> *mut Page {
    do_alloc_pages(flags, order)
}

/// Allocate single page.
///
/// **Note**: This function returns the **physical memory address** which is
/// aligned to the *page size* (4KiB).
///
/// **Call Convention**: See [the mod document].
///
/// [the mod document]: self
pub fn alloc_page(flags: usize) -> usize {
    let page = do_alloc_pages(flags, 0);
    page_to_address(page)
}

/// Allocate `2^order` number of pages (contiguous allocation).
///
/// **Note**: This function returns the **physical memory address** which is
/// aligned to the *page size* (4KiB).
///
/// **Call Convention**: See [the mod document].
///
/// [the mod document]: self
pub fn alloc_pages(flags: usize, order: usize) -> usize {
    let page = do_alloc_pages(flags, order);
    page_to_address(page)
}

/// Allocate and zero a page.
///
/// **Note**: This function returns the **physical memory address** which is
/// aligned to the *page size* (4KiB). The returned pages' memory has been
/// initialized with zero.
///
/// **Call Convention**: See [the mod document].
///
/// [the mod document]: self
pub fn alloc_zeroed_page(flags: usize) -> usize {
    let ret = alloc_page(flags);
    if ret != 0 {
        let size = PAGE_SIZE / 8;
        let big_ptr = ret as *mut u64;
        // big_ptr.write_bytes(0, size);
        for i in 0..size {
            // We use big_ptr so that we can force a sd (store doubleword)
            // instruction rather than the sb. This means 8x fewer than before.
            // Note that we won't have any remaining bytes because 4096 % 8 = 0.
            unsafe {
                (*big_ptr.add(i)) = 0;
            }
        }
    }

    ret
}

/// Free a single page.
pub fn return_page(page: *mut Page) {
    do_free_pages(page, 0);
}

/// Free an `order` number of pages from the given page.
pub fn return_pages(page: *mut Page, order: usize) {
    do_free_pages(page, order);
}

/// Free a page by its **physical address**.
///
/// **Call Convention**: See [the mod document].
///
/// [the mod document]: self
pub fn free_page(addr: usize) {
    if addr == 0 {
        debug_assert!(false);
        return;
    }

    let page = address_to_page(addr);
    do_free_pages(page, 0);
}

/// Free an `order` pages from the given **physical address**.
///
/// **Call Convention**: See [the mod document].
///
/// [the mod document]: self
pub fn free_pages(addr: usize, order: usize) {
    if addr == 0 {
        debug_assert!(false);
        return;
    }

    let page = address_to_page(addr);
    do_free_pages(page, order);
}

/// Get the **physical address** of a `page` struct.
pub fn page_address(page: *const Page) -> usize {
    page_to_address(page)
}


////////////////////// Inner Impl ///////////////////////////

fn page_to_address(page: *const Page) -> usize {
    unsafe {
        // core::intrinsics::unlikely()
        if page.is_null() {
            return 0;
        }

        let index = page.offset_from(PAGE_OBJ_BASE as _) as usize;
        ALLOC_START + index * PAGE_SIZE
    }
}

fn address_to_page(addr: usize) -> *mut Page {
    debug_assert!(addr.trailing_zeros() >= PAGE_ORDER as u32);
    unsafe {
        // core::intrinsics::unlikely()
        if addr <= ALLOC_START {
            return null_mut();
        }

        let index = (addr - ALLOC_START) / PAGE_SIZE;
        (PAGE_OBJ_BASE as *mut Page).add(index)
    }
}

fn do_alloc_pages(_flags: usize, order: usize) -> *mut Page {
    // todo: flags support.
    let size = 1usize << order;
    for zone_idx in 0..MAX_ZONE_COUNT {
        unsafe {
            let zone = MEMORY_ZONES.get_unchecked_mut(zone_idx);
            if size > zone.free_pages {
                continue;
            }

            // Try alloc on zone
            let page = alloc_page_on_zone(zone, order);
            if !page.is_null() {
                // Directly assign to clear the flags.
                (*page).flags = zone_idx as u32;
                // (*page).set_zone_idx(zone_idx);
                // Reset the ref count.
                (*page).ref_count.store(0, Ordering::Relaxed);
                return page;
            }
        }
    }

    null_mut()
}

#[inline(always)]
fn bitmap_mark_used(bitmap: *mut u8, index: usize, order: usize) {
    crate::util::bit::change_bit_array(bitmap, index >> (1usize + order));
}

unsafe fn alloc_page_on_zone(zone: &mut Zone, order: usize) -> *mut Page {
    for current_order in order..MAX_FREE_AREA_ORDER {
        let free_area = zone.free_areas.get_unchecked_mut(current_order);
        if list::is_empty(&free_area.free_list) {
            continue;
        }

        // list_entry
        let page_head = free_area.free_list.next;
        let page = crate::container_of_mut!(page_head, Page, head);
        list::delete(&mut *page_head);
        let index = page.offset_from(PAGE_OBJ_BASE as _) as usize;
        if current_order != MAX_FREE_AREA_ORDER - 1usize {
            bitmap_mark_used(free_area.bitmap, index, current_order);
        }

        zone.free_pages -= 1usize << order;
        expand_areas(page, index, order, current_order, free_area as _);
        return page;
    }

    null_mut()
}

unsafe fn expand_areas(page: *mut Page, index: usize, low: usize, mut high: usize, mut area: *mut FreeArea) {
    let mut size = 1usize << high;
    while low < high {
        high -= 1usize;
        area = area.sub(1);
        size >>= 1usize;
        let buddy = &mut (*page.add(size));
        buddy.flags = 0;
        list::head_append(&mut (*area).free_list, &mut buddy.head);
        bitmap_mark_used((*area).bitmap, index + size, high);
    }
}

fn do_free_pages(page: *mut Page, order: usize) {
    assert!(order < MAX_FREE_AREA_ORDER && !page.is_null());
    unsafe {
        let zone_idx = (*page).get_zone_idx();
        debug_assert!(zone_idx < MAX_ZONE_COUNT);

        let zone = MEMORY_ZONES.get_unchecked_mut(zone_idx);
        let area = zone.free_areas.get_unchecked_mut(order) as *mut FreeArea;
        free_pages_bulk(zone, page, area, order);
    }
}

unsafe fn free_pages_bulk(zone: &mut Zone, page: *mut Page, mut area: *mut FreeArea, order: usize) {
    let mut mask = !0usize << order;
    let base = PAGE_OBJ_BASE as *mut Page;
    let mut page_idx = page.offset_from(base) as usize;
    if (page_idx & !mask != 0) || (page_idx + 1usize << order > zone.max_pages) {
        panic!("Free page invalid.");
    }

    let mut index = page_idx >> (1usize + order);

    zone.free_pages += 1usize << order;
    for _ in order..(MAX_FREE_AREA_ORDER - 1usize) {
        if !crate::util::bit::test_and_change_bit_array((*area).bitmap, index) {
            break;
        }

        // Previous bit in bitmap is 1, so the buddy block is free, then do merge.
        let buddy = base.add(page_idx ^ (1usize << order));
        list::delete(&mut (*buddy).head);

        mask <<= 1usize;
        area = area.add(1);
        index >>= 1usize;
        page_idx &= mask;
    }
    list::head_append(&mut (*area).free_list, &mut (*base.add(page_idx)).head);
}


////////////////////// Debug Helper /////////////////////////////

/// Print all page allocations. Called from the M-mode or S-mode with identity PTE is set.
/// This is mainly used for debugging.
pub fn print_page_allocations() {
    unsafe {
        let zone = MEMORY_ZONES.get_unchecked(0);
        let num_pages = zone.max_pages;

        let heap_beg = super::HEAP_BASE;
        let heap_end = zone.mem_start + zone.mem_size;

        let beg = PAGE_OBJ_BASE as *const Page;
        let end = beg.add(num_pages);
        let alloc_beg = ALLOC_START;
        let alloc_end = ALLOC_START + num_pages * PAGE_SIZE;

        println_k!();
        println_k!(
            "PAGE ALLOCATION TABLE\nMETA: {:p} -> {:p}\nHEAP: 0x{:x} -> 0x{:x}\nPHYS: \
            0x{:x} -> 0x{:x}\nMEMORY BEGIN: {:#x}, SIZE: {:#x}",
            beg, end, heap_beg, heap_end, alloc_beg, alloc_end, zone.mem_start, zone.mem_size
        );
        println_k!("~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
        let mut num = 0;
        let mut order = 0usize;
        for free_area in &zone.free_areas {
            print_k!("FreeArea[{}]: ", order);

            let count = list::count(&free_area.free_list);
            if count == 0 {
                println_k!("<Empty>");
            } else {
                println_k!("{} item(s): {} << {} = {} page(s).", count, count, order, count << order);
            }

            num += count << order;
            order += 1usize;
        }

        println_k!("~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
        println_k!("Allocated: {:>5} pages ({:>9} bytes).", num_pages - num, (num_pages - num) * PAGE_SIZE);
        println_k!("Free     : {:>5} pages ({:>9} bytes).", num, num * PAGE_SIZE);
        println_k!();
    }
}
