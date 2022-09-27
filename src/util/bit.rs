//! Bit operations.

#[inline(always)]
pub const fn change_bit_u8(val: u8, pos: usize) -> u8 {
    val ^ ((1usize << pos) as u8)
}

#[inline]
pub fn change_bit_array(bits: *mut u8, pos: usize) {
    let byte_pos = pos / 8usize;
    let bits_pos = pos % 8usize;
    unsafe {
        let ptr = bits.add(byte_pos);
        ptr.write(change_bit_u8(ptr.read(), bits_pos));
    }
}
