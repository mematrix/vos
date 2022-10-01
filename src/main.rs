#![no_main]
#![no_std]
#![feature(default_alloc_error_handler)]    // GlobalAllocator need this.
#![feature(inline_const)]   // Needed in 'macros/ptr.rs'.
#![feature(const_ptr_offset_from)]

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
mod driver;
mod mm;
mod dev;
mod fs;
mod proc;
mod sc;

use core::arch::asm;
use log::{Log, Metadata, Record};

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

struct UartLogger;

impl Log for UartLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            if record.level() < log::Level::Info {
                println_k!("[{}][{}:{}]: {}",
                    record.level(),
                    record.file().unwrap_or("<NONE>"),
                    record.line().unwrap_or_default(),
                    record.args());
            } else {
                println_k!("[{}]: {}", record.level(), record.args());
            }
        }
    }

    fn flush(&self) {}
}

static UART_LOGGER: UartLogger = UartLogger;

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

    match log::set_logger(&UART_LOGGER) {
        Ok(_) => { log::set_max_level(log::LevelFilter::Trace); }
        Err(_) => { println_k!("Init set logger failed!"); }
    }

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

    // Create the first kernel thread: idle process with PID=0.
    let mut idle_task = proc::create_idle_kernel_thread();
    let frame = idle_task.get_trap_frame_ptr();
    arch::cpu::sscratch_write(frame as _);
    // Create the first user process: systemd process with PID=1. All other processes will
    // be forked from this.

    // Do schedule: We need first select a thread to run, write its `TrapFrame` into the
    // `sscratch` CSR, then set the next timer event and open the interrupt flag.
    // Set timer.
    arch::cpu::stimecmp_write_delta(10_000_000);    // 1s
    // Enable interrupt.
    arch::cpu::sstatus_sti();

    println_k!("Start typing, I'll show what you typed!");
    let uart = driver::uart::Uart::default();

    loop {
        if let Some(c) = uart.get() {
            match c {
                // 8 => {
                //     // This is a backspace, so we essentially have
                //     // to write a space and backup again:
                //     print_k!("{}{}{}", 8 as char, ' ', 8 as char);
                // },
                10 | 13 => {
                    // Newline or carriage-return
                    println_k!();
                }
                _ => {
                    print_k!("{}", (c as char).escape_default());
                }
            }
        } else {
            // unsafe {
            //     asm!("wfi");
            // }
        }
    }
}
