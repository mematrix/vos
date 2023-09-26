//! CPU interrupt mask handling.

use crate::arch::cpu;

/// Enable the IRQ of current CPU core.
#[inline(always)]
pub fn local_irq_enable() {
    cpu::sstatus_sti();
}

/// Disable the IRQ of current CPU core.
#[inline(always)]
pub fn local_irq_disable() {
    cpu::sstatus_cli();
}

/// Save the current IRQ enable state.
#[inline(always)]
pub fn local_save_flags() -> usize {
    cpu::sstatus_read()
}

/// Given the `flags`, test if IRQ is disabled.
#[inline(always)]
pub fn is_irq_disabled_flags(flags: usize) -> bool {
    cpu::check_irq_disabled_flags(flags)
}

/// Check if the current CPU's IRQ is disabled.
#[inline(always)]
pub fn is_irq_disabled() -> bool {
    cpu::is_irq_disabled()
}

/// Disable the current IRQ and return the last IRQ state.
#[inline(always)]
pub fn local_irq_save() -> usize {
    cpu::sstatus_cli_save()
}

/// Restore saved IRQ state.
#[inline(always)]
pub fn local_irq_restore(flags: usize) {
    cpu::sstatus_write(flags);
}
