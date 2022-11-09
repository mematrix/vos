//! Functions and macros for accessing and manipulating `preempt_count` (used for kernel
//! preemption, interrupt count, etc.).
//!
//! The `preempt_count` exists in the task struct [`TaskInfo`], so any functions or macros
//! in this mod **must** be called after the tasks have been start scheduling.

use core::borrow::Borrow;
use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::mem::forget;
use core::ops::{Deref, DerefMut};
use crate::arch::cpu;
use crate::barrier;
use crate::proc::kernel::ctx::{self_task_info, self_task_info_mut};
use crate::proc::task::TaskInfo;
use crate::sched::preempt_schedule;

/*
 * **Ref from the Linux (include/linux/preempt.h)**
 * We put the hardirq and softirq counter into the preemption
 * counter. The bitmask has the following meaning:
 *
 * - bits 0-7 are the preemption count (max preemption depth: 256)
 * - bits 8-15 are the softirq count (max # of softirqs: 256)
 *
 * The hardirq count could in theory be the same as the number of
 * interrupts in the system, but we run all interrupt handlers with
 * interrupts disabled, so we cannot have nesting interrupts. Though
 * there are a few palaeontologic drivers which reenable interrupts in
 * the handler, so we need more than one bit here.
 *
 *         PREEMPT_MASK:	0x000000ff
 *         SOFTIRQ_MASK:	0x0000ff00
 *         HARDIRQ_MASK:	0x000f0000
 *             NMI_MASK:	0x00f00000
 * PREEMPT_NEED_RESCHED:	0x80000000
 */
const PREEMPT_BITS: i32 = 8;
const SOFTIRQ_BITS: i32 = 8;
const HARDIRQ_BITS: i32 = 4;
const NMI_BITS: i32 = 4;

const PREEMPT_SHIFT: i32 = 0;
const SOFTIRQ_SHIFT: i32 = PREEMPT_SHIFT + PREEMPT_BITS;
const HARDIRQ_SHIFT: i32 = SOFTIRQ_SHIFT + SOFTIRQ_BITS;
const NMI_SHIFT: i32 = HARDIRQ_SHIFT + HARDIRQ_BITS;

const fn __irq_mask(bits: i32) -> u32 {
    (1u32 << bits) - 1u32
}

pub const PREEMPT_MASK: u32 = __irq_mask(PREEMPT_BITS) << PREEMPT_SHIFT;
pub const SOFTIRQ_MASK: u32 = __irq_mask(SOFTIRQ_BITS) << SOFTIRQ_SHIFT;
pub const HARDIRQ_MASK: u32 = __irq_mask(HARDIRQ_BITS) << HARDIRQ_SHIFT;
pub const NMI_MASK: u32 = __irq_mask(NMI_BITS) << NMI_SHIFT;

pub const PREEMPT_OFFSET: u32 = 1u32 << PREEMPT_SHIFT;
pub const SOFTIRQ_OFFSET: u32 = 1u32 << SOFTIRQ_SHIFT;
pub const HARDIRQ_OFFSET: u32 = 1u32 << HARDIRQ_SHIFT;
pub const NMI_OFFSET: u32 = 1u32 << NMI_SHIFT;

pub const SOFTIRQ_DISABLE_OFFSET: u32 = 2 * SOFTIRQ_OFFSET;
pub const PREEMPT_DISABLE_OFFSET: u32 = PREEMPT_OFFSET;

pub const PREEMPT_NEED_RESCHED: u64 = 1u64 << 32;
pub const PREEMPT_ENABLED: u64 = PREEMPT_NEED_RESCHED;
pub const PREEMPT_DISABLED: u64 = PREEMPT_ENABLED + PREEMPT_DISABLE_OFFSET as u64;

pub const FORK_PREEMPT_COUNT: u64 = PREEMPT_ENABLED + 2 * PREEMPT_DISABLE_OFFSET as u64;

/// Read the current preempt count.
#[inline(always)]
pub fn preempt_count() -> u32 {
    read_once!(self_task_info().preempt_union.preempt.count)
}

#[inline(always)]
pub fn set_preempt_count(pc: u32) {
    write_once!(self_task_info_mut().preempt_union.preempt.count, pc);
}

#[inline(always)]
pub fn init_task_preempt_count(p: &mut TaskInfo) {
    p.preempt_union.preempt_count = FORK_PREEMPT_COUNT;
}

#[inline(always)]
pub fn init_idle_preempt_count(p: &mut TaskInfo) {
    p.preempt_union.preempt_count = PREEMPT_DISABLED;
}

#[inline(always)]
pub fn preempt_set_need_resched() {
    self_task_info_mut().preempt_union.preempt.need_resched = 0;
}

#[inline(always)]
pub fn preempt_clear_need_resched() {
    self_task_info_mut().preempt_union.preempt.need_resched = 1;
}

#[inline(always)]
pub fn preempt_test_need_resched() -> bool {
    unsafe {
        self_task_info().preempt_union.preempt.need_resched == 0
    }
}

#[inline]
pub fn preempt_count_add(v: u32) {
    let current = self_task_info_mut();
    let mut pc = read_once!(current.preempt_union.preempt.count);
    pc += v;
    write_once!(current.preempt_union.preempt.count, pc);
}

#[inline]
pub fn preempt_count_sub(v: u32) {
    let current = self_task_info_mut();
    let mut pc = read_once!(current.preempt_union.preempt.count);
    pc -= v;
    write_once!(current.preempt_union.preempt.count, pc);
}

#[inline]
pub fn preempt_count_dec_and_test() -> bool {
    let current = self_task_info_mut();
    let mut pc = read_once!(current.preempt_union.preempt_count);

    pc -= 1u64;
    // Update only the `count` field, leaving `need_resched` unchanged.
    write_once!(current.preempt_union.preempt.count, pc as u32);

    // If we wrote back all zeroes, then we're preemptible and in need of a reschedule.
    // Otherwise, we need to reload the preempt_count in case the need_resched flag was
    // cleared by an interrupt occurring between the non-atomic READ_ONCE/WRITE_ONCE pair.
    pc == 0 || read_once!(current.preempt_union.preempt_count) == 0
}

#[inline(always)]
pub fn should_resched(preempt_offset: u64) -> bool {
    read_once!(self_task_info().preempt_union.preempt_count) == preempt_offset
}


/////////////////// Preempt Operations ////////////////////

#[inline(always)]
pub fn preempt_disable() {
    preempt_count_add(1);
    barrier!();
}

#[inline(always)]
pub fn preempt_enable_no_resched() {
    barrier!();
    preempt_count_sub(1);
}

#[inline(always)]
pub fn preempt_enable() {
    barrier!();
    if preempt_count_dec_and_test() {
        preempt_schedule();
    }
}

#[inline(always)]
pub fn preempt_check_resched() {
    if should_resched(0) {
        preempt_schedule();
    }
}

#[inline(always)]
pub fn preemptible() -> bool {
    (preempt_count() == 0) && !cpu::is_irq_disabled()
}


////////////////////// Context Check //////////////////////

/// Returns the current interrupt context level.
///
/// * 0 - normal context
/// * 1 - softirq context
/// * 2 - hardirq context
/// * 3 - NMI context
#[inline(always)]
pub fn interrupt_context_level() -> u32 {
    let pc = preempt_count();
    let mut level: u32 = 0;

    level += ((pc & NMI_MASK) != 0) as u32;
    level += ((pc & (NMI_MASK | HARDIRQ_MASK)) != 0) as u32;
    level += ((pc & (NMI_MASK | HARDIRQ_MASK | SOFTIRQ_MASK)) != 0) as u32;

    level
}

#[inline(always)]
pub fn nmi_count() -> u32 { preempt_count() & NMI_MASK }

#[inline(always)]
pub fn hardirq_count() -> u32 { preempt_count() & HARDIRQ_MASK }

#[inline(always)]
pub fn softirq_count() -> u32 { preempt_count() & SOFTIRQ_MASK }

#[inline(always)]
pub fn irq_count() -> u32 { preempt_count() & (NMI_MASK | HARDIRQ_MASK | SOFTIRQ_MASK) }

/*
 * Helpers to retrieve the current execution context:
 */
/// If we're in NMI context.
#[inline(always)]
pub fn in_nmi() -> bool { nmi_count() != 0 }

/// If we're in hard IRQ context.
#[inline(always)]
pub fn in_hardirq() -> bool { hardirq_count() != 0 }

/// If we're in soft IRQ context.
#[inline(always)]
pub fn in_serving_softirq() -> bool { (softirq_count() & SOFTIRQ_OFFSET) != 0 }

/// If we're in task context.
#[inline(always)]
pub fn in_task() -> bool { !(in_nmi() | in_hardirq() | in_serving_softirq()) }


///////////////////// Helper Objects //////////////////////

/// A simple wrapper that guards a value is accessed in the **preempt-disabled** context.
pub struct PreemptGuard<T> {
    value: T,
}

impl<T> PreemptGuard<T> {
    /// Init the `PreemptGuard` object with an initialized value. **Note that the initializing
    /// process of `value` that out of this method call may not be protected by disabling
    /// the preemption.**
    #[inline(always)]
    pub fn new(value: T) -> Self {
        preempt_disable();
        Self {
            value
        }
    }

    /// Init the `PreemptGuard` object that the inner value is initialized with the supplier.
    /// The supplier function is called in the **preempt-disabled** context.
    #[inline(always)]
    pub fn init_by<F>(f: F) -> Self
        where F: FnOnce() -> T {
        preempt_disable();
        Self {
            value: f(),
        }
    }

    pub fn map<U, F>(self, f: F) -> PreemptGuard<U>
        where F: FnOnce(&T) -> U {
        let ret = PreemptGuard {
            value: f(&self.value)
        };
        forget(self);
        ret
    }
}

impl<T> Borrow<T> for PreemptGuard<T> where T: Eq + Ord + Hash {
    #[inline(always)]
    fn borrow(&self) -> &T {
        &self.value
    }
}

impl<T: PartialEq> PartialEq for PreemptGuard<T> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&other.value)
    }
}

impl<T: Eq> Eq for PreemptGuard<T> {}

impl<T: PartialOrd> PartialOrd for PreemptGuard<T> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<T: Ord> Ord for PreemptGuard<T> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T: Hash> Hash for PreemptGuard<T> {
    #[inline(always)]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T> AsRef<T> for PreemptGuard<T> {
    #[inline(always)]
    fn as_ref(&self) -> &T {
        &self.value
    }
}

impl<T> AsMut<T> for PreemptGuard<T> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T> Deref for PreemptGuard<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for PreemptGuard<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

impl<T> Drop for PreemptGuard<T> {
    #[inline(always)]
    fn drop(&mut self) {
        // no sched?
        preempt_enable();
    }
}
