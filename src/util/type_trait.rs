//! Compile time type check traits.

use core::marker::PhantomData;
use core::mem::size_of;


pub trait IsTrue {}

pub trait IsFalse {}

pub enum Assert<const COND: bool> {}

impl IsTrue for Assert<true> {}

impl IsFalse for Assert<false> {}

pub struct TypeSizeEq<T, const SIZE: usize> {
    _phantom: PhantomData<T>,
}

impl<T, const SIZE: usize> IsTrue for TypeSizeEq<T, SIZE> where Assert<{size_of::<T>() == SIZE}>: IsTrue {}

impl<T, const SIZE: usize> IsFalse for TypeSizeEq<T, SIZE> where Assert<{size_of::<T>() == SIZE}>: IsFalse {}

pub const fn is_native_word_length(s: usize) -> bool {
    s == size_of::<u8>() || s == size_of::<u16>() || s == size_of::<u32>() || s == size_of::<u64>()
}

pub struct IsNativeWord<T> {
    _phantom: PhantomData<T>,
}

impl<T> IsTrue for IsNativeWord<T> where Assert<{is_native_word_length(size_of::<T>())}>: IsTrue {}

impl<T> IsFalse for IsNativeWord<T> where Assert<{is_native_word_length(size_of::<T>())}>: IsFalse {}
