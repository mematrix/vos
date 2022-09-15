//! Kernel memory management for sub-page level: malloc-like allocation system.

use core::{mem::size_of, ptr::null_mut};
use crate::mm::{align_val, page::{PAGE_SIZE, zalloc}};


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
pub fn init() {
    unsafe {
        // Allocate 512 kernel pages (512 * 4KiB = 2MiB)
        const ALLOC_COUNT: usize = 512;
        let k_alloc = zalloc(ALLOC_COUNT);
        debug_assert!(k_alloc != 0);
        let k_alloc = k_alloc as *mut AllocList;
        (*k_alloc).set_free();
        (*k_alloc).set_size(ALLOC_COUNT * PAGE_SIZE);

        KMEM_ALLOC = ALLOC_COUNT;
        KMEM_HEAD = k_alloc;
    }
}

/// Allocate sub-page level allocation based on bytes.
///
/// If the function successfully allocates a memory, the memory is guaranteed to be aligned
/// to 8 bytes.
pub fn kmalloc(sz: usize) -> *mut u8 {
    if sz == 0 {
        return null_mut();
    }

    unsafe {
        let size = align_val(sz, 3) + size_of::<AllocList>();
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
pub fn kzmalloc(sz: usize) -> *mut u8 {
    let size = align_val(sz, 3);
    let ret = kmalloc(size);

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

/////////////// GLOBAL ALLOCATOR //////////////////////

// The global allocator allows us to use the data structures in the core library, such
// as a linked list or B-tree.
use core::alloc::{GlobalAlloc, Layout};

// The global allocator is a static constant to a global allocator
// structure. We don't need any members because we're using this
// structure just to implement alloc and dealloc.
struct OsGlobalAlloc;

unsafe impl GlobalAlloc for OsGlobalAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // We align to the next page size so that when
        // we divide by PAGE_SIZE, we get exactly the number
        // of pages necessary.
        kzmalloc(layout.size())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        // We ignore layout since our allocator uses ptr_start -> last
        // to determine the span of an allocation.
        kfree(ptr);
    }
}

#[global_allocator]
static GA: OsGlobalAlloc = OsGlobalAlloc {};

// If for some reason alloc() in the global allocator gets null_mut(), then we come here.
// This is a divergent function, so we call panic to let the tester know what's going on.
// #[alloc_error_handler]
// pub fn alloc_error(l: Layout) -> ! {
//     panic!(
//         "Allocator failed to allocate {} bytes with {}-byte alignment.",
//         l.size(),
//         l.align()
//     );
// }
