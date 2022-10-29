mod riscv;

pub use riscv::*;


/// Compiler barrier, disable the compiler re-ordering across this point.
#[macro_export]
macro_rules! barrier {
    () => {
        ::core::sync::atomic::compiler_fence(::core::sync::atomic::Ordering::SeqCst);
    };
}
