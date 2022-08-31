mod boot_asm;
mod trap_asm;
mod mem_asm;

pub(crate) mod mem_v {
    #[no_mangle]
    extern "C" {
        static HEAP_START: usize;
        static HEAP_SIZE: usize;
        static TEXT_START: usize;
        static TEXT_END: usize;
        static DATA_START: usize;
        static DATA_END: usize;
        static RODATA_START: usize;
        static RODATA_END: usize;
        static BSS_START: usize;
        static BSS_END: usize;
        static KERNEL_STACK_START: usize;
        static KERNEL_STACK_END: usize;
        static mut KERNEL_TABLE: usize;
    }
}
