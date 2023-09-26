//! Re-export a subset APIs of other modules that will be used in the `base` module.
//!
//! * [`sched`]
//!
//! [`sched`]: crate::sched

pub(super) mod sched_api {
    pub use crate::sched::{preempt_disable, preempt_enable};
}

