#![no_main]
#![no_std]
#![feature(default_alloc_error_handler)]    // GlobalAllocator need this.
#![feature(inline_const)]   // Needed in 'macros/ptr.rs'.
#![feature(const_refs_to_cell)]     // An negative error reported by v1.66.0-nightly

#[macro_use]
extern crate log;
extern crate static_assertions as sa;
extern crate alloc;

mod asm;
#[macro_use]
mod macros;
mod util;
mod constant;

mod arch;
mod init;
mod logk;
mod driver;
mod smp;
mod mm;
mod dev;
mod fs;
mod proc;
mod sched;

use core::arch::asm;


// #[lang = "eh_personality"]
// extern fn eh_personality() {}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println_k!("{}", info);
    // if let Some(p) = info.location() {
    //     println_k!(
    //         "Aborting: line {}, file {}: <todo panic message>",
    //         p.line(),
    //         p.file()
    //     );
    // } else {
    //     println_k!("Aborting: no information available.");
    // }
    abort();
}

#[no_mangle]
extern "C"
fn abort() -> ! {
    loop {
        unsafe {
            asm!("wfi");
        }
    }
}

//////// TEST offset_of! //////////////

#[derive(Default)]
struct RustOffsetTest {
    pub _padding: u8,
    pub align: u32,
    pub align64: u64,
    pub align8: u8,
}

#[repr(C)]
#[derive(Default)]
struct COffsetTest {
    pub _padding: u8,
    pub align: u32,
    pub align64: u64,
    pub align8: u8,
}

#[repr(C, packed)]
#[derive(Default)]
struct COffsetPackedTest {
    pub _padding: u8,
    pub align: u32,
    pub align64: u64,
    pub align8: u8,
}

#[repr(C, packed(4))]
#[derive(Default)]
struct COffsetAlignTest {
    pub _padding: u8,
    pub align: u32,
    pub align64: u64,
    pub align8: u8,
}

///////////// END TEST ////////////////

#[no_mangle]
/// Do initialization on the machine mode (CPU mode #3).
/// Returns the SATP value (including the MODE).
extern "C"
fn m_init(hart_id: usize, dtb: *const u8) -> usize {
    let satp = init::boot_setup(dtb);

    println_k!("Hello Rust OS");
    println_k!("Running in hart#{}, dtb: {:p}", hart_id, dtb);

    satp
}

#[no_mangle]
extern "C"
fn kmain() {
    // Main should initialize all sub-systems and get
    // ready to start scheduling. The last thing this
    // should do is start the timer.

    init::kernel_setup();

    println_k!();
    println_k!("Now we are in the Supervisor mode.");
    println_k!();

    macro_rules! show_offset_test {
        ($ty:tt) => {{
            let off_test: $ty = Default::default();
            println_k!("Show info for type: {}", stringify!($ty));
            println_k!(" * size_of: {}", core::mem::size_of::<$ty>());
            println_k!(" * align_of: {}", core::mem::align_of::<$ty>());
            println_k!(" * offset/align: {}", offset_of!($ty, align));
            println_k!(" * offset/align64: {}", offset_of!($ty, align64));
            println_k!(" * offset/align8: {}", offset_of!($ty, align8));
            println_k!(" * ptr: {:p}", &off_test);
            let ptr_a32 = core::ptr::addr_of!(off_test.align);
            let ptr = unsafe { container_of!(ptr_a32, $ty, align) };
            println_k!(" * ptr/align: {:p}, container: {:p}", ptr_a32, ptr);
            let ptr_a64 = core::ptr::addr_of!(off_test.align64);
            let ptr = unsafe { container_of!(ptr_a64, $ty, align64) };
            println_k!(" * ptr/align64: {:p}, container: {:p}", ptr_a64, ptr);
            let ptr_a8 = core::ptr::addr_of!(off_test.align8);
            let ptr = unsafe { container_of!(ptr_a8, $ty, align8) };
            println_k!(" * ptr/align8: {:p}, container: {:p}", ptr_a8, ptr);
            println_k!();
        }};
    }

    show_offset_test!(RustOffsetTest);
    show_offset_test!(COffsetTest);
    show_offset_test!(COffsetPackedTest);
    show_offset_test!(COffsetAlignTest);

    // Create the first kernel thread: idle process with TID=0 (All kernel thread has a PID of 0).
    proc::init();
    sched::init();

    // Add the kernel test threads.
    proc::add_test_kernel_threads();

    // Create the first user process: systemd process with PID=1. All other processes will
    // be forked from this.

    // Do schedule: We need first select a thread to run, write its `TrapFrame` into the
    // `sscratch` CSR, then set the next timer event and open the interrupt flag.
    sched::schedule();
    // The `schedule` will never return.
}
