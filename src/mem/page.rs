//! Page-based memory management.
//!
//! This mod provides some functions to allocate/deallocate the physical memory
//! from the HEAP area. The page size is default set to 4KiB.

// todo: add page alloc fn for discontinuous pages: fn a(s: usize, c: fn(*mut()), u: *mut())

use core::{mem::size_of, ptr::null_mut};

use super::align_val;
use crate::asm::mem_v::{HEAP_START, HEAP_SIZE};


// We will use ALLOC_START to mark the start of the actual
// memory we can dish out.
static mut ALLOC_START: usize = 0;
// Track the max number than can be allocated.
static mut ALLOC_PAGES: usize = 0;

const PAGE_ORDER: usize = 12;
pub const PAGE_SIZE: usize = 1 << 12;

#[repr(u8)]
enum PageBits {
    Empty = 0,
    Taken = 1 << 0,
    Last = 1 << 1,

    LastTaken = 1 << 0 | 1 << 1,
}

impl PageBits {
    /// Get the underlying representation value.
    #[inline]
    pub const fn val(self) -> u8 {
        self as u8
    }
}

/// Each page is described by the Page structure.
struct Page {
    flags: u8,
}

impl Page {
    /// Check if this page has been marked as the final allocation.
    #[inline]
    pub fn is_last(&self) -> bool {
        self.flags & PageBits::Last.val() != 0
    }

    /// Check if this page is marked as being taken (allocated).
    #[inline]
    pub fn is_taken(&self) -> bool {
        self.flags & PageBits::Taken.val() != 0
    }

    /// Check if this page is **not** marked as being taken (not allocated).
    #[inline]
    pub fn is_free(&self) -> bool {
        !self.is_taken()
    }

    /// Clear the `Page` structure, marked as unused.
    #[inline]
    pub fn clear(&mut self) {
        self.flags = PageBits::Empty.val();
    }

    /// Set a certain flag.
    #[inline]
    pub fn set_flag(&mut self, flag: PageBits) {
        self.flags |= flag.val();
    }

    /// Clear a certain flag.
    #[inline]
    pub fn clear_flag(&mut self, flag: PageBits) {
        self.flags &= !(flag.val());
    }
}

/// Initialize the page-based allocation system.
pub fn init() {
    unsafe {
        let size = HEAP_SIZE;
        let start = HEAP_START;
        let num_pages = size / PAGE_SIZE;
        let ptr = start as *mut Page;
        // Determine where the actual useful memory starts. This will be after all
        // Page structures. We also must align the ALLOC_START to a page-boundary
        // (PAGE_SIZE = 4096).
        let alloc_start = align_val(start + num_pages * size_of::<Page>(), PAGE_ORDER);
        // Then we need compute the actual pages count that can be allocated,
        // because the Page descriptors are also allocated on the HEAP start address
        // and will take some pages of the memory.
        let actual_pages = (size - (alloc_start - start)) / PAGE_SIZE;
        // Clear all pages to make sure that they can be allocated.
        for i in 0..actual_pages {
            (*ptr.add(i)).clear();
        }
        // Bytes in actual_pages..num_pages are wasted, its value is about
        // HEAP_SIZE / (PAGE_SIZE * PAGE_SIZE). Which means 1 byte is wasted
        // for every roughly 16MiB of the memory.

        ALLOC_START = alloc_start;
        ALLOC_PAGES = actual_pages;
    }
}

/// Allocate a page or multiple pages (contiguous allocation).
/// `pages` is the number of Page to allocate.
pub fn alloc(pages: usize) -> *mut u8 {
    assert!(pages > 0);
    unsafe {
        let num_pages = ALLOC_PAGES;
        let ptr = HEAP_START as *mut Page;
        let mut i = 0usize;
        while i < (num_pages - pages) {
            // Check to see if this Page is free. If so, we have the first
            // candidate memory address.
            if (*ptr.add(i)).is_free() {
                let mut found = true;
                for j in (i + 1)..(i + pages) {
                    // Now check to see if we have a contiguous allocation
                    // for all of the request pages.
                    if (*ptr.add(j)).is_taken() {
                        found = false;
                        // Move scan position to skip the range i..j because
                        // we have checked the pages in this range.
                        i = j;
                        break;
                    }
                }

                if found {
                    // Now we get the enough contiguous pages to form that we need.
                    for k in i..(i + pages - 1) {
                        (*ptr.add(k)).set_flag(PageBits::Taken);
                    }
                    // Mark the last page is PageBits::Last.
                    (*ptr.add(i + pages - 1)).set_flag(PageBits::LastTaken);

                    // The Page structures themselves aren't the useful memory.
                    // Instead, there is 1 page per 4096 bytes starting at ALLOC_START.
                    return (ALLOC_START + PAGE_SIZE * i) as *mut u8;
                }
            }

            // Move scan position to next.
            i += 1;
        }
    }

    null_mut()
}

/// Allocate and zero a page or multiple pages (contiguous allocation).
/// `pages` is the number of Page to allocate.
pub fn zalloc(pages: usize) -> *mut u8 {
    let ret = alloc(pages);
    if !ret.is_null() {
        let size = (pages * PAGE_SIZE) / 8;
        let big_ptr = ret as *mut u64;
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

/// Deallocate a page by its pointer.
pub fn dealloc(ptr: *mut u8) {
    // The way we structure this, it will automatically coalesce contiguous pages.
    debug_assert!(!ptr.is_null());
    if ptr.is_null() {
        return;
    }

    unsafe {
        let page_id = (ptr as usize - ALLOC_START) / PAGE_SIZE;
        // Make sure that the page id (index) makes sense.
        assert!(page_id < ALLOC_PAGES);

        let mut p = HEAP_START as *mut Page;
        p = p.add(page_id);
        while (*p).is_taken() && !(*p).is_last() {
            (*p).clear();
            p = p.add(1);
        }

        // If the following assertion fails, it is most likely caused by a
        // double-free.
        assert!((*p).is_last(), "Possible double-free detected! (Not taken found before last)");

        // If we get here, we've taken care of all previous pages and we are
        // on the last page.
        (*p).clear();
    }
}

/// Print all page allocations
/// This is mainly used for debugging.
pub fn print_page_allocations() {
    unsafe {
        let num_pages = ALLOC_PAGES;
        let heap_beg = HEAP_START;
        let heap_end = heap_beg + HEAP_SIZE;
        let mut beg = HEAP_START as *const Page;
        let end = beg.add(num_pages);
        let alloc_beg = ALLOC_START;
        let alloc_end = ALLOC_START + num_pages * PAGE_SIZE;
        println_k!();
        println_k!(
            "PAGE ALLOCATION TABLE\nMETA: {:p} -> {:p}\nHEAP: 0x{:x} -> 0x{:x}\nPHYS: \
            0x{:x} -> 0x{:x}",
            beg, end, heap_beg, heap_end, alloc_beg, alloc_end
        );
        println_k!("~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
        let mut num = 0;
        while beg < end {
            if (*beg).is_taken() {
                let start = beg as usize;
                let memaddr = alloc_beg + (start - heap_beg) / size_of::<Page>() * PAGE_SIZE;
                print_k!("0x{:x} => ", memaddr);
                loop {
                    num += 1;
                    if (*beg).is_last() {
                        let end = beg as usize;
                        let memaddr = alloc_beg + (end - heap_beg) / size_of::<Page>() * PAGE_SIZE + PAGE_SIZE - 1;
                        print_k!("0x{:x}: {:>3} page(s)", memaddr, ((end - start) / size_of::<Page>() + 1));
                        println_k!(".");
                        break;
                    }
                    beg = beg.add(1);
                }
            }
            beg = beg.add(1);
        }

        println_k!("~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~");
        println_k!("Allocated: {:>5} pages ({:>9} bytes).", num, num * PAGE_SIZE);
        println_k!("Free     : {:>5} pages ({:>9} bytes).", num_pages - num, (num_pages - num) * PAGE_SIZE);
        println_k!();
    }
}
