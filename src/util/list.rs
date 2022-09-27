//! Circular doubly linked list implementation.

use core::ptr::null_mut;


/// Doubly linked list. Embedded in the actual entry struct to give the entry struct
/// the linked list capability.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct List {
    pub prev: *mut List,
    pub next: *mut List,
}

impl List {
    /// Construct a default `List` object with its `prev` and `next` pointer are all null. Any list
    /// API call on this object is **Undefined Behavior**. [`init_empty`] or other init operations
    /// must be called before any other list operations.
    ///
    /// [`init_empty`]: self::List::init_empty
    pub const fn new() -> Self {
        Self {
            prev: null_mut(),
            next: null_mut(),
        }
    }

    /// Init the list object by making an empty list.
    #[inline(always)]
    pub fn init_empty(&mut self) {
        let this = self as *mut List;
        self.prev = this;
        self.next = this;
    }

    /// Get the next list entry.
    #[inline(always)]
    pub const fn next(self) -> *mut List {
        self.next
    }

    /// Get the previous list entry.
    #[inline(always)]
    pub const fn prev(self) -> *mut List {
        self.prev
    }
}

/// Append a new list `entry` on the list tail.
#[inline(always)]
pub fn tail_append(head: &mut List, entry: &mut List) {
    insert_before(head, entry);
}

/// Append a new list `entry` on the list head.
#[inline(always)]
pub fn head_append(head: &mut List, entry: &mut List) {
    insert_after(head, entry);
}

/// Append `entry` after the list `node`, but only do a partial operations: `node.next` set to `entry`,
/// `entry.prev` set to `node`; and the original *next entry* of `node` did not been changed, the
/// `entry.next` value also does not been changed.
///
/// # Safety
/// Caller must guard the final list is in valid state by some sequence calls of this function.
pub unsafe fn partial_append(node: &mut List, entry: &mut List) {
    node.next = entry as _;
    entry.prev = node as _;
}

/// Insert a new list `entry` before the `node`.
#[inline]
pub fn insert_before(node: &mut List, entry: &mut List) {
    entry.prev = node.prev;
    entry.next = node as _;
    unsafe { (*node.prev).next = entry as _; }
    node.prev = entry as _;
}

/// Insert a new list `entry` after the `node`.
#[inline]
pub fn insert_after(node: &mut List, entry: &mut List) {
    entry.prev = node as _;
    entry.next = node.next;
    unsafe { (*node.next).prev = entry as _; }
    node.next = entry as _;
}

/// Test whether a list is empty.
#[inline(always)]
pub fn is_empty(head: &List) -> bool {
    (head as *const List) == head.next
}

/// Check whether the `entry` is the last item of list `head`.
///
/// **Note**: This function will return `true` if `head` and `entry` are the same empty list.
#[inline(always)]
pub fn is_last(head: &List, entry: &List) -> bool {
    (entry as *const List) == head.prev
}

/// Test whether a list has just one entry.
#[inline(always)]
pub fn is_singular(head: &List) -> bool {
    (head as *const List) != head.next && (head.next == head.prev)
}

/// Delete `entry` from list. No effect if list is empty.
///
/// After delete, the `entry` object is in an undefined state.
#[inline(always)]
pub fn delete(entry: &mut List) {
    unsafe {
        (*entry.prev).next = entry.next;
        (*entry.next).prev = entry.prev;
    }
}

/// Delete `entry` from list and init the `entry` to empty state.
#[inline]
pub fn delete_and_init_empty(entry: &mut List) {
    delete(entry);
    entry.init_empty();
}

/// Count the list items.
pub fn count(head: &List) -> usize {
    let mut count = 0usize;
    let ptr = head as *const List;
    let mut cur = head;
    while ptr != cur.next {
        count += 1usize;
        cur = unsafe { &*cur.next }
    }

    count
}
