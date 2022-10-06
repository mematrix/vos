//! Utilities to build the kernel thread.

use core::mem::size_of;
use core::sync::atomic::{AtomicU32, Ordering};
use crate::arch::cpu::{self, Register};
use crate::mm::{page, kfree, kzalloc, get_satp_identity_map, PAGE_SIZE};
use crate::proc::task::{TaskInfo, TaskType};


/// Kernel thread entry function signature.
pub type ThreadEntry = extern "C" fn(*mut ()) -> usize;

/// Build a kernel thread object.
pub fn build_kernel_thread(entry: ThreadEntry, user_data: *mut ()) -> ThreadBuilder {
    // todo: use Result?
    ThreadBuilder::new(entry, user_data).unwrap()
}


/// Kernel tid allocation counter.
static KERNEL_TID: AtomicU32 = AtomicU32::new(0);

pub struct ThreadBuilder {
    task_info: &'static mut TaskInfo,
}

impl ThreadBuilder {
    pub fn new(entry: ThreadEntry, user_data: *mut ()) -> Option<Self> {
        let ptr = kzalloc(size_of::<TaskInfo>(), 0);
        if ptr.is_null() {
            return None;
        }

        // todo: use vmalloc to get a virtual address protection.
        // Kernel thread has a stack size of 2^2 pages, 16KiB.
        let stack = page::alloc_pages(0, 2);    // todo: const val = 2
        if stack == 0 {
            kfree(ptr);
            return None;
        }

        let ret = Self {
            task_info: unsafe { &mut *(ptr as *mut TaskInfo) },
        };
        ret.task_info.set_tid(KERNEL_TID.fetch_add(1, Ordering::AcqRel));
        ret.task_info.set_task_type(TaskType::Kernel);

        let frame = ret.task_info.trap_frame_mut();
        // On kernel thread, the `kernel_stack` points to the stack memory.
        frame.kernel_stack = stack as _;
        frame.satp = get_satp_identity_map();
        // todo: set mode,
        // Set `epc` to default kernel thread entry.
        frame.pc = start_kernel_thread as *const () as usize;
        // Then set the `entry` and `user_data` as the parameters.
        unsafe {
            // SAFETY: reg index is guarded within the ranges.
            let regs = &mut frame.regs;
            *regs.get_unchecked_mut(cpu::reg(Register::A0)) = entry as *const () as _;
            *regs.get_unchecked_mut(cpu::reg(Register::A1)) = user_data as _;
            *regs.get_unchecked_mut(cpu::reg(Register::A2)) = ptr as _;
            // Set thread stack. Stack is growing from high to low address.
            let top = stack + PAGE_SIZE * (1usize << 2) - size_of::<usize>();
            *regs.get_unchecked_mut(cpu::reg(Register::Sp)) = top;
        }

        Some(ret)
    }
}

#[allow(dead_code)]
extern "C"
fn start_kernel_thread(entry: ThreadEntry, user_data: *mut (), task_info: &mut TaskInfo) /* -> ! */ {
    // Before run.
    // Start run task.
    let ret = entry(user_data);
    info!("Kernel thread (tid = {}) finished, return {}.", task_info.tid(), ret);

    // todo: scheduler do last schedule and thread die work.
    task_info.set_exit_code(ret);
}
