#![no_main]
#![no_std]

mod asm;
#[macro_use]
mod macros;

mod driver;
mod mem;

use core::arch::asm;

/* QEMU RISC-V memory maps (qemu/hw/riscv/virt.c)
static const MemMapEntry virt_memmap[] = {
    [VIRT_DEBUG] =        {        0x0,         0x100 },
    [VIRT_MROM] =         {     0x1000,        0xf000 },
    [VIRT_TEST] =         {   0x100000,        0x1000 },
    [VIRT_RTC] =          {   0x101000,        0x1000 },
    [VIRT_CLINT] =        {  0x2000000,       0x10000 },
    [VIRT_ACLINT_SSWI] =  {  0x2F00000,        0x4000 },
    [VIRT_PCIE_PIO] =     {  0x3000000,       0x10000 },
    [VIRT_PLATFORM_BUS] = {  0x4000000,     0x2000000 },
    [VIRT_PLIC] =         {  0xc000000, VIRT_PLIC_SIZE(VIRT_CPUS_MAX * 2) },
    [VIRT_APLIC_M] =      {  0xc000000, APLIC_SIZE(VIRT_CPUS_MAX) },
    [VIRT_APLIC_S] =      {  0xd000000, APLIC_SIZE(VIRT_CPUS_MAX) },
    [VIRT_UART0] =        { 0x10000000,         0x100 },
    [VIRT_VIRTIO] =       { 0x10001000,        0x1000 },
    [VIRT_FW_CFG] =       { 0x10100000,          0x18 },
    [VIRT_FLASH] =        { 0x20000000,     0x4000000 },
    [VIRT_IMSIC_M] =      { 0x24000000, VIRT_IMSIC_MAX_SIZE },
    [VIRT_IMSIC_S] =      { 0x28000000, VIRT_IMSIC_MAX_SIZE },
    [VIRT_PCIE_ECAM] =    { 0x30000000,    0x10000000 },
    [VIRT_PCIE_MMIO] =    { 0x40000000,    0x40000000 },
    [VIRT_DRAM] =         { 0x80000000,           0x0 },
};
*/

// #[lang = "eh_personality"]
// extern fn eh_personality() {}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    if let Some(p) = info.location() {
        println_k!(
            "Aborting: line {}, file {}: <todo panic message>",
            p.line(),
            p.file()
        );
    } else {
        println_k!("Aborting: no information available.");
    }
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
/// Do initialization on the machine mode (CPU mode #3)
extern "C"
fn m_init() {
    let uart = driver::uart::Uart::default();
    uart.init_default();

    println_k!("Hello Rust OS");
    println_k!("Start typing, I'll show what you typed!");

}

#[no_mangle]
extern "C"
fn kmain() {
    // Main should initialize all sub-systems and get
    // ready to start scheduling. The last thing this
    // should do is start the timer.

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
