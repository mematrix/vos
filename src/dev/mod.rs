//! Device definitions.

pub mod pm;

use crate::driver::Driver;


#[repr(C)]
pub struct Device {
    pub(crate) init_name: &'static str,
    pub(crate) driver: Option<&'static dyn Driver>,
    pub driver_data: *mut (),
}
