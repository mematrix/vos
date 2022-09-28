//! Provides some helper methods to do alignment.

use core::mem::align_of;


/// Get the order of an alignment value. Which is the count of the trailing zero bits
/// of the `alignment`.
///
/// **Note**: The `alignment` value **must** be a power of 2, otherwise the result is
/// incorrect. Type align info can be retrieved by [`core::mem::align_of`].
///
/// [`core::mem::align_of`]: ::core::mem::align_of
#[inline(always)]
pub const fn get_order(alignment: usize) -> usize {
    alignment.trailing_zeros() as usize
}

/// Returns the **aligned** value of `val`.
///
/// An **aligned** value is guaranteed that the least bits (width is specified
/// by `order`) are set to zero. Therefore, all alignments must be made as a
/// power of two.
///
/// This function always rounds up. So the returned value will always be
/// **not less than** the `val`.
#[inline(always)]
pub const fn align_up(val: usize, order: usize) -> usize {
    let o = (1usize << order) - 1;
    (val + o) & !o
}

/// Returns the **aligned** value of `val`. Similar to [`align_up`], but this
/// function aligns value by rounding down, it will simple set the least `order` bits
/// to zero. So the returned value will always be **not greater than** the `val`.
///
/// [`align_up`]: self::align_up
#[inline(always)]
pub const fn align_down(val: usize, order: usize) -> usize {
    let o = (1usize << order) - 1;
    val & !o
}

/// Returns the **aligned** value of `val`. Use the alignment of type `T`.
///
/// This function always rounds up. See [`align_up`].
///
/// [`align_up`]: self::align_up
#[inline(always)]
pub const fn align_up_of<T>(val: usize) -> usize {
    // Type alignment is guaranteed be a power of 2.
    let order = align_of::<T>().trailing_zeros();
    align_up(val, order as usize)
}

/// Returns the **aligned** value of `val`. Use the alignment of type `T`.
///
/// This function always rounds down. See [`align_down`].
///
/// [`align_down`]: self::align_down
#[inline(always)]
pub const fn align_down_of<T>(val: usize) -> usize {
    let order = align_of::<T>().trailing_zeros();
    align_down(val, order as usize)
}
