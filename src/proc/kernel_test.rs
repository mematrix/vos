//! Some test purpose kernel threads.

use crate::arch::cpu::read_time;
use crate::driver::uart::Uart;
use crate::proc::kernel::build_kernel_thread;
use crate::sched::ready_list_add_task;
use crate::smp::current_cpu_info;


pub fn add_test_kernel_threads() {
    let cur_cpu = current_cpu_info();

    // uart echo test thread
    let task = build_kernel_thread(uart_echo_test, 0x10000000usize as _).build();
    ready_list_add_task(task);

    // simple timer.
    let timebase = cur_cpu.get_timebase_freq();
    let time_4s = timebase << 2;
    let task = build_kernel_thread(simple_timer_test, time_4s as _).build();
    ready_list_add_task(task);
}

extern "C"
fn uart_echo_test(uart_addr: *mut ()) -> usize {
    let uart = Uart::new(uart_addr as _);
    info!("[UartTest] Open uart device @{:p}.", uart_addr);

    println_k!("[UartTest] Start typing, I'll show what you typed!");
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
        }
    }
}

extern "C"
fn simple_timer_test(interval_clock: *mut ()) -> usize {
    let interval = interval_clock as usize;
    info!("[TimerTest] Start a timer with interval clock@{}", interval);

    let mut time = read_time();
    info!("[TimerTest] Timer start at clock@{}", time);
    loop {
        let cur = read_time();
        if cur >= time + interval {
            info!("[TimerTest] Trigger timer at clock@{}", cur);
            time = cur;
        }
    }
}
