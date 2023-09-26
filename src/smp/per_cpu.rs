//! A common way to define the per-cpu data struct.

use core::mem::size_of;
use core::ptr::{null_mut, slice_from_raw_parts_mut};
use crate::mm::{kfree, kmalloc};
use crate::sched::{PreemptGuard, PreemptValueProvider};
use crate::smp::current_cpu_info;
use super::CPU_COUNT;


/// Define the per-cpu struct. Use `get_*` to read current cpu data with implied preemption
/// protection, and use `get_*_raw` to do operations without preemption guard.
#[repr(C)]
pub struct PerCpuPtr<T: 'static> {
    data_array: *mut T,
}

impl<T: 'static> PerCpuPtr<T> {
    /// Create an empty ptr without any objects. A [`init`] or [`init_with_ptr`] method **must**
    /// be called before using this ptr.
    ///
    /// [`init`]: PerCpuPtr::init
    /// [`init_with_ptr`]: PerCpuPtr::init_with_ptr
    #[inline(always)]
    pub const fn null() -> Self {
        Self {
            data_array: null_mut()
        }
    }

    /// Create new object for each cpu. **This method can be called only after the initialization
    /// of the SLAB/SLUB allocator and `kmalloc` is available**.
    pub fn new() -> Self {
        let ptr: *mut T;
        unsafe {
            let bytes = CPU_COUNT * size_of::<T>();
            ptr = kmalloc(bytes, 0) as _;
        }

        Self {
            data_array: ptr,
        }
    }

    /// Create a map for each cpu with the external data array.
    ///
    /// # Safety
    ///
    /// The ptr `array_ptr` **must** point to an array of `T` and have a size of **at least** the
    /// number of `CPU_COUNT`.
    #[inline(always)]
    pub const unsafe fn new_with_ptr(array_ptr: *mut T) -> Self {
        Self {
            data_array: array_ptr,
        }
    }

    /// Init the memory for each cpu. **This method can be called only after the initialization
    /// of the SLAB/SLUB allocator and `kmalloc` is available**.
    #[inline]
    pub fn init(&mut self) {
        unsafe {
            let bytes = CPU_COUNT * size_of::<T>();
            self.data_array = kmalloc(bytes, 0) as _;
        }
    }

    /// Init the per-cpu map memory with the external data array.
    ///
    /// # Safety
    ///
    /// The ptr `array_ptr` **must** point to an array of `T` and have a size of **at least** the
    /// number of `CPU_COUNT`.
    #[inline(always)]
    pub const unsafe fn init_with_ptr(&mut self, array_ptr: *mut T) {
        self.data_array = array_ptr;
    }

    /// Get the data ptr of *current cpu*. **Note that on the preemption-enabled context, the
    /// returned pointer is not guaranteed to be associated with the cpu that use it**. See
    /// [`get`] to read the per-cpu data with the guard of preemption disabled.
    ///
    /// [`get`]: PerCpuPtr::get
    #[inline]
    pub fn get_raw(&self) -> *mut T {
        let cur_cpu_id = current_cpu_info().get_cpu_id();
        unsafe {
            self.data_array.add(cur_cpu_id)
        }
    }

    /// Get the data ref of *current cpu*. **Note that on the preemption-enabled context, the
    /// returned pointer is not guaranteed to be associated with the cpu that use it**. See
    /// [`get_ref`] to read the per-cpu data with the guard of preemption disabled.
    ///
    /// [`get_ref`]: PerCpuPtr::get_ref
    pub fn get_ref_raw(&self) -> &T {
        unsafe {
            &*self.get_raw()
        }
    }

    /// Get mut ref of data of *current cpu*. **Note that on the preemption-enabled context, the
    /// returned pointer is not guaranteed to be associated with the cpu that use it**. See
    /// [`get_ref_mut`] to read the per-cpu data with the guard of preemption disabled.
    ///
    /// [`get_ref_mut`]: PerCpuPtr::get_ref_mut
    pub fn get_ref_mut_raw(&self) -> &mut T {
        unsafe {
            &mut *self.get_raw()
        }
    }

    /// Get the data ptr of current cpu.
    #[inline(always)]
    pub fn get(&self) -> PreemptGuard<*mut T> {
        PreemptGuard::init_by(|| self.get_raw())
    }

    /// Get the data ref of current cpu.
    #[inline(always)]
    pub fn get_ref(&self) -> PreemptGuard<&T> {
        PreemptGuard::init_by(|| self.get_ref_raw())
    }

    /// Get mut ref of data of current cpu.
    #[inline(always)]
    pub fn get_ref_mut(&self) -> PreemptGuard<&mut T> {
        PreemptGuard::init_by(|| self.get_ref_mut_raw())
    }

    /// Get all objects as an array. The array length is equal to `CPU_COUNT`.
    #[inline(always)]
    pub fn as_array_mut(&self) -> &mut [T] {
        unsafe { &mut *slice_from_raw_parts_mut(self.data_array, CPU_COUNT) }
    }

    /// Release the memory hold by this object.
    ///
    /// **Note**: **only** the object created by [`Self::new`] or constructed with [`init`] can
    /// be destroyed with this method. It is **Undefined Behavior** if the object was created
    /// by [`new_with_ptr`] or init by [`init_with_ptr`].
    ///
    /// [`Self::new`]: PerCpuPtr::new
    /// [`init`]: PerCpuPtr::init
    /// [`new_with_ptr`]: PerCpuPtr::new_with_ptr
    /// [`init_with_ptr`]: PerCpuPtr::init_with_ptr
    #[inline]
    pub fn destroy(this: &mut Self) {
        kfree(this.data_array as _);
        this.data_array = null_mut();
    }
}
