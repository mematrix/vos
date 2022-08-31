pub(crate) mod page;


/// Returns the **aligned** value of `val`.
///
/// An **aligned** value is guaranteed that the least bits (width is specified
/// by `order`) are set to zero. Therefore, all alignments must be made as a
/// power of two.
///
/// This function always rounds up. So the returned value will always be
/// **not less than** the `val`.
pub const fn align_val(val: usize, order: usize) -> usize {
    let o = (1usize << order) - 1;
    (val + o) & !o
}
