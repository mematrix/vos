//! Provides the spin-lock implementation.

use core::sync::atomic::{AtomicBool, Ordering};


/// A spin lock object works like the C type, it only provides the lock semantic but
/// does not manage any data.
#[repr(C)]
pub struct SpinLockPure {
    lock: AtomicBool,
}

impl SpinLockPure {
    #[inline]
    pub const fn new() -> Self {
        Self {
            lock: AtomicBool::new(false),
        }
    }

    #[inline]
    pub fn lock(&self) {
        while self.lock.compare_exchange_weak(
            false, true, Ordering::AcqRel, Ordering::Relaxed).is_err() {}
    }

    #[inline]
    pub fn unlock(&self) {
        self.lock.store(false, Ordering::Release);
    }

    #[inline]
    pub fn lock_guard(&self) -> SpinLockPureGuard {
        self.lock();
        SpinLockPureGuard {
            lock: self
        }
    }
}

pub struct SpinLockPureGuard<'a> {
    lock: &'a SpinLockPure,
}

impl<'a> Drop for SpinLockPureGuard<'a> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}
