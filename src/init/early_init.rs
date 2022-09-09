//! Do initializations on the early boot time.

use core::ptr::copy_nonoverlapping;

use fdt::standard_nodes::Chosen;
use super::{BOOT_COMMAND_LINE};


pub fn dt_scan_chosen(chosen: &Chosen) {
    if let Some(args) = chosen.bootargs() {
        unsafe {
            copy_nonoverlapping(args.as_ptr(), BOOT_COMMAND_LINE.as_mut_ptr(), BOOT_COMMAND_LINE.len());
        }
    }
}
