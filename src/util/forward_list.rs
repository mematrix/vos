//! Singly linked-list (Intrusive) implementation.

use core::ptr::null_mut;


#[repr(C)]
#[derive(Copy, Clone)]
pub struct ForwardList {
    next: *mut ForwardList,
}

impl ForwardList {
    /// Construct an empty forward linked-list whose `next` pointer is null.
    #[inline(always)]
    pub const fn new_empty() -> Self {
        Self {
            next: null_mut(),
        }
    }

    /// Returns `true` if list is empty.
    #[inline(always)]
    pub const fn is_empty(self) -> bool {
        self.next.is_null()
    }

    /// Get the next list entry.
    #[inline(always)]
    pub const fn next(self) -> *mut ForwardList {
        self.next
    }
}

#[inline(always)]
pub fn insert_after(head: &mut ForwardList, next: &mut ForwardList) {
    next.next = head.next;
    head.next = next as _;
}

#[inline(always)]
pub fn remove_next(head: &mut ForwardList) {
    debug_assert!(!head.next.is_null());
    unsafe {
        head.next = (*head.next).next;
    }
}
