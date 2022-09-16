//! Init CPU info from the DT data.

use core::num::NonZeroI32;
use core::ptr::null_mut;
use crate::dev::Device;
use super::{Metadata, Driver};
use super::of::DeviceId;


struct CpuDriver {
    metadata: Metadata,
    match_table: &'static [DeviceId],
}

impl Driver for CpuDriver {
    fn get_metadata(&self) -> &Metadata {
        &self.metadata
    }

    fn get_match_table(&self) -> Option<&[DeviceId]> {
        Some(self.match_table)
    }

    fn probe(&self, _dev: &mut Device) -> Result<(), NonZeroI32> {
        Ok(())
    }

    fn remove(&self, dev: &mut Device) -> Result<(), NonZeroI32> {
        dev.driver_data = null_mut();
        Ok(())
    }
}

static CPU_DRIVER: CpuDriver = CpuDriver {
    metadata: Metadata::with_name("cpu"),
    match_table: &[DeviceId::with_compat("riscv")],
};

pub fn export_driver() -> &'static dyn Driver {
    &CPU_DRIVER
}
