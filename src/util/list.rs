//! Circular doubly linked list implementation.

/// Double linked list. Embedded in the actual entry struct to give the entry struct
/// the linked list capability.
#[repr(C)]
pub struct List {
    pub prev: *mut List,
    pub next: *mut List,
}
