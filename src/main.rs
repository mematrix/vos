#![no_main]
#![no_std]

mod asm;
#[macro_use]
mod macros;

mod driver;
mod mem;

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
    println_k!("Start typing, I'll show what you typed!");

    let fdt = unsafe { fdt::Fdt::from_ptr(dtb) };
    if let Ok(fdt) = fdt {
        println_k!("Success to parse the device tree blob.");
        println_k!(" * Device tree total size: {} bytes.", fdt.total_size());
        println_k!(" * Root:");
        let root = fdt.root();
        println_k!("   * CellSize: {:?}", root.cell_sizes());
        println_k!("   * Model: {}", root.model());
        println_k!("   * Compatibility: {}", root.compatible().first());
        println_k!("   * Properties:");
        for p in root.properties() {
            println_k!("     * {}: str({}), usize({} ({2:#x}))", p.name,
                p.as_str().unwrap_or(""), p.as_usize().unwrap_or_default());
        }

        println_k!(" * Memory:");
        let memory = fdt.memory();
        if let Some(init_map) = memory.initial_mapped_area() {
            println_k!("   * initial-mapped-area: {:?}", init_map);
        }
        for r in memory.regions() {
            println_k!("   * region: starting_addr={:p}, size={:#x} ({1})",
                r.starting_address, r.size.unwrap_or_default());
        }

        println_k!(" * Chosen:");
        let chosen = fdt.chosen();
        println_k!("   * bootargs: {}", chosen.bootargs().unwrap_or("no args"));

        println_k!(" * Aliases:");
        if let Some(aliases) = fdt.aliases() {
            for a in aliases.all() {
                println_k!("   * {} -> {}", a.0, a.1);
            }
        }

        println_k!(" * CPUs:");
        for (id, cpu) in fdt.cpus().enumerate() {
            println_k!("   * cpu#{}:", id);
            println_k!("     * ids: {}", cpu.ids().first());
            for p in cpu.properties() {
                println_k!("     * {}: str({}), usize({} ({2:#x}))", p.name,
                    p.as_str().unwrap_or(""), p.as_usize().unwrap_or_default());
            }
        }

        println_k!();
        println_k!("////////// dump device tree (name) ////////////");
        if let Some(node) = fdt.find_node("/") {
            print_node(node, 0);
        }
    } else {
        println_k!("Parsing the dtb failed!");
    }

    0
}

fn print_node(node: fdt::node::FdtNode<'_, '_>, n_spaces: usize) {
    (0..n_spaces).for_each(|_| print_k!(" "));
    println_k!("{}/", node.name);

    for child in node.children() {
        print_node(child, n_spaces + 2);
    }
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
