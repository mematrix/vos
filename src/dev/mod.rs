//! Device definitions.

pub(crate) mod cpu;
pub mod pm;

use crate::driver::Driver;


#[repr(C)]
pub struct Device {
    pub(crate) init_name: &'static str,
    pub(crate) driver: Option<&'static dyn Driver>,
    pub driver_data: *mut (),
}


/// Init some device data after the DeviceTree has been un-flattened.
pub fn init(cpu_count: usize) {
    cpu::init_smp(cpu_count);
}
