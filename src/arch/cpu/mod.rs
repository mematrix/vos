mod riscv;

pub use riscv::*;


/// If the IRQ is disabled, return `true`, otherwise return `false`.
#[inline(always)]
pub fn is_irq_disabled() -> bool {
    (sstatus_read() & 0b0010usize) == 0
}
