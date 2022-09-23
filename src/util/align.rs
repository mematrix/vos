//! Provides some helper methods to do alignment.

use core::mem::align_of;


/// Returns the **aligned** value of `val`.
///
/// An **aligned** value is guaranteed that the least bits (width is specified
/// by `order`) are set to zero. Therefore, all alignments must be made as a
/// power of two.
///
/// This function always rounds up. So the returned value will always be
/// **not less than** the `val`.
pub const fn align_val_up(val: usize, order: usize) -> usize {
    let o = (1usize << order) - 1;
    (val + o) & !o
}

/// Returns the **aligned** value of `val`. Similar to [`align_val_up`], but this
/// function aligns value by rounding down, it will simple set the least `order` bits
/// to zero. So the returned value will always be **not greater than** the `val`.
///
/// [`align_val_up`]: self::align_val_up
pub const fn align_val_down(val: usize, order: usize) -> usize {
    let o = (1usize << order) - 1;
    val & !o
}

/// Returns the **aligned** value of `val`. Use the alignment of type `T`.
///
/// This function always rounds up. See [`align_val_up`].
///
/// [`align_val_up`]: self::align_val_up
pub const fn align_up_of<T>(val: usize) -> usize {
    // Type alignment is guaranteed be a power of 2.
    let order = align_of::<T>().trailing_zeros();
    align_val_up(val, order as usize)
}

/// Returns the **aligned** value of `val`. Use the alignment of type `T`.
///
/// This function always rounds down. See [`align_val_down`].
///
/// [`align_val_down`]: self::align_val_down
pub const fn align_down_of<T>(val: usize) -> usize {
    let order = align_of::<T>().trailing_zeros();
    align_val_down(val, order as usize)
}
