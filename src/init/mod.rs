//! Kernel initialization operation and data.

pub const COMMAND_LINE_SIZE: usize = 256;

/// Untouched command line saved by arch-special code.
pub static mut BOOT_COMMAND_LINE: [u8; COMMAND_LINE_SIZE] = [0u8; COMMAND_LINE_SIZE];
