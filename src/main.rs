#![no_main]
#![no_std]
#![feature(default_alloc_error_handler)]

#[macro_use]
extern crate log;
extern crate static_assertions as sa;
extern crate alloc;

mod asm;
#[macro_use]
mod macros;

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

#[no_mangle]
/// Do initialization on the machine mode (CPU mode #3).
/// Returns the SATP value (including the MODE).
extern "C"
fn m_init(hart_id: usize, dtb: *const u8) -> usize {
    let uart = driver::uart::Uart::default();
    uart.init_default();

    match log::set_logger(&UART_LOGGER) {
        Ok(_) => { log::set_max_level(log::LevelFilter::Trace); }
        Err(_) => { println_k!("Init set logger failed!"); }
    }

    println_k!("Hello Rust OS");
    println_k!("Running in hart#{}, dtb: {:p}", hart_id, dtb);

    let fdt = unsafe { driver::of::fdt::parse_from_ptr(dtb) };
    driver::of::fdt::show_fdt_standard_nodes(&fdt);
    driver::of::fdt::dump_fdt(&fdt);

    init::early_setup(&fdt)
}

#[no_mangle]
extern "C"
fn kmain() {
    // Main should initialize all sub-systems and get
    // ready to start scheduling. The last thing this
    // should do is start the timer.

    println_k!();
    println_k!("Now we are in the Supervisor mode.");
    println_k!();
    init::setup();

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
