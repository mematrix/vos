//! Provides the spin-lock implementation.

use core::sync::atomic::{AtomicBool, Ordering};

use crate::base::import::sched_api;
use crate::base::irq;


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
    pub fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Acquire)
    }

    #[inline]
    pub fn try_lock(&self) -> bool {
        self.lock.compare_exchange_weak(false, true, Ordering::AcqRel, Ordering::Relaxed).is_ok()
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
        raw_spin_lock(self);
        SpinLockPureGuard {
            lock: self
        }
    }

    #[inline]
    pub fn lock_guard_irq(&self) -> SpinLockPureGuardIrq {
        raw_spin_lock_irq(self);
        SpinLockPureGuardIrq {
            lock: self
        }
    }

    #[inline]
    pub fn lock_guard_irq_save(&self) -> SpinLockPureGuardSaveIrq {
        let flags = raw_spin_lock_irq_save(self);
        SpinLockPureGuardSaveIrq {
            lock: self,
            flags
        }
    }
}

pub struct SpinLockPureGuard<'a> {
    lock: &'a SpinLockPure,
}

impl<'a> Drop for SpinLockPureGuard<'a> {
    fn drop(&mut self) {
        raw_spin_unlock(self.lock);
    }
}

pub struct SpinLockPureGuardIrq<'a> {
    lock: &'a SpinLockPure,
}

impl<'a> Drop for SpinLockPureGuardIrq<'a> {
    fn drop(&mut self) {
        raw_spin_unlock_irq(self.lock);
    }
}

pub struct SpinLockPureGuardSaveIrq<'a> {
    lock: &'a SpinLockPure,
    flags: usize
}

impl<'a> Drop for SpinLockPureGuardSaveIrq<'a> {
    fn drop(&mut self) {
        raw_spin_unlock_irq_restore(self.lock, self.flags);
    }
}


#[inline]
pub fn raw_spin_lock(lock: &SpinLockPure) {
    sched_api::preempt_disable();
    lock.lock();
}

#[inline]
pub fn raw_spin_lock_irq(lock: &SpinLockPure) {
    irq::local_irq_disable();
    sched_api::preempt_disable();
    lock.lock();
}

#[inline]
pub fn raw_spin_lock_irq_save(lock: &SpinLockPure) -> usize {
    let flags = irq::local_irq_save();
    sched_api::preempt_disable();
    lock.lock();
    flags
}

#[inline]
pub fn raw_spin_try_lock(lock: &SpinLockPure) -> bool {
    sched_api::preempt_disable();
    if lock.try_lock() {
        true
    } else {
        sched_api::preempt_enable();
        false
    }
}

#[inline]
pub fn raw_spin_unlock(lock: &SpinLockPure) {
    lock.unlock();
    sched_api::preempt_enable();
}

#[inline]
pub fn raw_spin_unlock_irq(lock: &SpinLockPure) {
    lock.unlock();
    irq::local_irq_enable();
    sched_api::preempt_enable();
}

#[inline]
pub fn raw_spin_unlock_irq_restore(lock: &SpinLockPure, flags: usize) {
    lock.unlock();
    irq::local_irq_restore(flags);
    sched_api::preempt_enable();
}
