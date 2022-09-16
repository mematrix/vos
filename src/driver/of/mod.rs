pub(crate) mod fdt;

/// Struct used for matching a device.
/// An **empty string** of `name`, `ty`, and `compatible` represents an any match.
pub struct DeviceId {
    pub name: &'static str,
    // type
    pub ty: &'static str,
    pub compatible: &'static str,
}

impl DeviceId {
    /// Helper method to construct a `DeviceId` object with empty `name` and `ty` and the
    /// specific `compatible`.
    pub const fn with_compat(compatible: &'static str) -> Self {
        Self {
            name: "",
            ty: "",
            compatible
        }
    }
}

/// Device node definition of the DeviceTree.
pub struct DeviceNode {
    //
}
