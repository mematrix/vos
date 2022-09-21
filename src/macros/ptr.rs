//! Define some macros like the C `offsetof`, `container_of` macros. The implementation refers
//! to the [Zhihu article](https://zhuanlan.zhihu.com/p/526894770).

/// The macro `offset_of` expands to an integral constant expression of type `usize`, the value
/// of which is the offset, in bytes, from the beginning of an object of specified type to its
/// specified subobject, including padding if any.
#[macro_export]
macro_rules! offset_of {
    ($ty:path, $field:tt) => {
        const {
            #[allow(
                unused_unsafe,
                clippy::as_conversions,
                clippy::unneeded_field_pattern,
                clippy::undocumented_unsafe_blocks
            )]
            unsafe {
                use ::core::mem::MaybeUninit;
                use ::core::primitive::{u8, usize};
                use ::core::ptr;

                // ensure the type is a named struct
                // ensure the field exists and is accessible
                let $ty { $field: _, .. };

                // const since 1.36
                let uninit: MaybeUninit<$ty> = MaybeUninit::uninit();

                // const since 1.59
                let base_ptr: *const $ty = uninit.as_ptr();

                // stable since 1.51
                let field_ptr: *const _ = ptr::addr_of!((*base_ptr).$field);

                // feature(const_ptr_offset_from)
                let base_addr = base_ptr.cast::<u8>();
                let field_addr = field_ptr.cast::<u8>();
                field_addr.offset_from(base_addr) as usize
            }
        }
    };
}

/// Cast a member of a structure out to the containing structure.
///
/// - `ptr`: the pointer to the member.
/// - `ty`: the type of the container struct this is embedded in.
/// - `member`: the name of the member within the struct.
#[macro_export]
macro_rules! container_of {
    ($ptr:expr, $ty:path, $field:tt) => {
        use ::core::primitive::u8;
        let ptr: *const _ = $ptr;
        ptr.cast::<u8>().sub(offset_of!($ty, $field)).cast::<$ty>()
    };
}

/// Cast a member of a structure out to the containing structure. Similar to [`container_of!`],
/// but this macro returns a **mut** ptr.
///
/// - `ptr`: the pointer to the member.
/// - `ty`: the type of the container struct this is embedded in.
/// - `member`: the name of the member within the struct.
///
/// [`container_of!`]: container_of
#[macro_export]
macro_rules! container_of_mut {
    ($ptr:expr, $ty:path, $field:tt) => {
        use ::core::primitive::u8;
        let ptr: *mut _ = $ptr;
        ptr.cast::<u8>().sub(offset_of!($ty, $field)).cast::<$ty>()
    };
}
