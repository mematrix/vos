//! Synchronization primitives.

mod spin_lock;


pub mod lock {
    pub use super::spin_lock::*;
}
