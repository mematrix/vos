#![no_main]
#![no_std]

mod asm;
#[macro_use]
mod macros;

mod arch;
mod init;
mod driver;
mod mem;
mod dev;
mod proc;
mod sc;

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

#[no_mangle]
/// Do initialization on the machine mode (CPU mode #3).
/// Returns the SATP value (including the MODE).
extern "C"
fn m_init(hart_id: usize, dtb: *const u8) -> usize {
    let uart = driver::uart::Uart::default();
    uart.init_default();

    println_k!("Hello Rust OS");
    println_k!("Running in hart#{}, dtb: {:p}", hart_id, dtb);

    let fdt = unsafe { driver::fdt::parse_from_ptr(dtb) };
    driver::fdt::show_fdt_standard_nodes(&fdt);
    driver::fdt::dump_fdt(&fdt);

    init::early_setup(&fdt)
}

#[no_mangle]
extern "C"
fn kmain() {
    // Main should initialize all sub-systems and get
    // ready to start scheduling. The last thing this
    // should do is start the timer.

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
                },
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
