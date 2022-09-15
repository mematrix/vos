//! Power management.

/// Power-management message.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct PmMessage {
    pub event: i32,
}

impl PmMessage {
    pub const fn new(event: i32) -> Self {
        Self {
            event
        }
    }
}
