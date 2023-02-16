pub(crate) mod boot;
pub(crate) mod of;
pub(crate) mod uart;
pub(crate) mod cpu;

use core::num::NonZeroI32;
use crate::dev::{Device, pm::PmMessage};


pub struct Metadata {
    pub name: &'static str,
    pub mod_name: &'static str,
    // bus type: core::cell::Cell<>
    // dev_pm_ops
}

impl Metadata {
    #[inline]
    pub const fn new(name: &'static str, mod_name: &'static str) -> Self {
        Self {
            name,
            mod_name,
        }
    }

    #[inline]
    pub const fn with_name(name: &'static str) -> Self {
        Self::new(name, "")
    }
}

pub trait Driver {
    fn get_metadata(&self) -> &Metadata;

    fn get_match_table(&self) -> Option<&[of::DeviceId]>;
    // fn get_acpi_match_table(&self)

    fn probe(&self, dev: &mut Device) -> Result<(), NonZeroI32>;

    fn remove(&self, dev: &mut Device) -> Result<(), NonZeroI32>;

    fn shutdown(&self, _dev: &mut Device) {}

    fn suspend(&self, _dev: &mut Device, _state: PmMessage) -> Result<(), NonZeroI32> {
        Ok(())
    }

    fn resume(&self, _dev: &mut Device) -> Result<(), NonZeroI32> {
        Ok(())
    }
}
