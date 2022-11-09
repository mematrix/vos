//! A common way to define the per-cpu data struct.

use core::mem::size_of;
use core::ptr::{null_mut, slice_from_raw_parts_mut};
use crate::mm::{kfree, kmalloc};
use crate::sched::PreemptGuard;
use crate::smp::current_cpu_info;
use super::CPU_COUNT;


pub struct PerCpuPtr<T> {
    data_array: *mut T,
}

impl<T> PerCpuPtr<T> {
    /// Create an empty ptr without any objects. A [`init`] method **must** be called before
    /// using this ptr.
    ///
    /// [`init`]: self::PerCpuPtr::init
    /// [`init_with_ptr`]: self::PerCpuPtr::init_with_ptr
    pub const fn new_empty() -> Self {
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
    pub fn init(&mut self) {
        unsafe {
            let bytes = CPU_COUNT * size_of::<T>();
            self.data_array = kmalloc(bytes, 0) as _;
        }
    }

    /// Get the data ptr of current cpu. **This method must be called on the preemption-disabled
    /// context**.
    pub fn get_raw(&self) -> *mut T {
        let cur_cpu_id = current_cpu_info().get_cpu_id();
        unsafe {
            self.data_array.add(cur_cpu_id)
        }
    }

    /// Get the data ptr of current cpu.
    pub fn get(&self) -> PreemptGuard<*mut T> {
        PreemptGuard::init_by(|| self.get_raw())
    }

    /// Get the data ref of current cpu.
    #[inline(always)]
    pub fn get_ref(&self) -> PreemptGuard<&T> {
        PreemptGuard::init_by(|| unsafe {
            &*self.get_raw()
        })
    }

    /// Get mut ref of data of current cpu.
    #[inline(always)]
    pub fn get_ref_mut(&self) -> PreemptGuard<&mut T> {
        PreemptGuard::init_by(|| unsafe {
            &mut *self.get_raw()
        })
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
    /// by [`new_with_ptr`].
    ///
    /// [`Self::new`]: self::PerCpuPtr::new
    /// [`init`]: self::PerCpuPtr::init
    /// [`new_with_ptr`]: self::PerCpuPtr::new_with_ptr
    pub fn destroy(this: &mut Self) {
        kfree(this.data_array as _);
        this.data_array = null_mut();
    }
}
