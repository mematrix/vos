//! Memory barrier. Provides the wrapper of `fence` instruction for the common **Acquire**
//! and **Release** semantics.

use core::sync::atomic::{compiler_fence, Ordering};


/// These barriers need to enforce ordering on both devices or memory.
#[macro_export]
macro_rules! mb {
    () => {
        ::core::sync::atomic::compiler_fence(::core::sync::atomic::Ordering::SeqCst);
        $crate::fence!("iorw", "iorw");
    };
    (r) => {
        ::core::sync::atomic::compiler_fence(::core::sync::atomic::Ordering::Acquire);
        $crate::fence!("ir", "ir");
    };
    (w) => {
        ::core::sync::atomic::compiler_fence(::core::sync::atomic::Ordering::Release);
        $crate::fence!("ow", "ow");
    };
}

/// These barriers do not need to enforce ordering on devices, just memory.
#[macro_export]
macro_rules! smp_mb {
    () => {
        ::core::sync::atomic::compiler_fence(::core::sync::atomic::Ordering::SeqCst);
        $crate::fence!("rw", "rw");
    };
    (r) => {
        ::core::sync::atomic::compiler_fence(::core::sync::atomic::Ordering::Acquire);
        $crate::fence!("r", "r");
    };
    (w) => {
        ::core::sync::atomic::compiler_fence(::core::sync::atomic::Ordering::Release);
        $crate::fence!("w", "w");
    };
}

#[inline(always)]
pub fn smb_store_release<T>(p: &mut T, v: T)
    where
        crate::IsNativeWord<T>: crate::IsTrue {
    compiler_fence(Ordering::Release);
    crate::fence!("rw", "w");
    unsafe {
        (p as *mut T).write_volatile(v);
    }
}

#[inline(always)]
pub fn smb_load_acquire<T>(p: &T) -> T
    where
        crate::IsNativeWord<T>: crate::IsTrue {
    let v = unsafe { (p as *const T).read_volatile() };
    compiler_fence(Ordering::Acquire);
    crate::fence!("r", "rw");
    v
}
