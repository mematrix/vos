//! Handle scheduler request.

use crate::proc::kernel::build_idle_thread;
use crate::proc::task::{TaskInfo, TaskStatus};
use crate::smp::PerCpuPtr;
use crate::util::list;
use crate::util::list::List;


pub(super) fn init_and_set_idle_task() {
    unsafe {
        TASK_LIST.ready_head.init_empty();
        // todo: add idle task to ready_head with the lowest priority.
        TASK_LIST.cpu_idle.init();
        let all_cpu_data = TASK_LIST.cpu_idle.as_array_mut();
        for cpu_idle in all_cpu_data {
            build_idle_thread(cpu_idle as _);
        }
    }
}

/// Find a `Ready` status task, return the idle task if no ready task.
pub(super) fn find_ready_task_or_idle() -> *mut TaskInfo {
    let task_list = unsafe { &TASK_LIST };

    if list::is_empty(&task_list.ready_head) {
        task_list.cpu_idle.get()
    } else {
        unsafe {
            // Remove the first ready task from `ready_head`.
            let next = task_list.ready_head.next;
            let task_info = container_of_mut!(next, TaskInfo, list);
            list::delete(&mut *next);

            task_info
        }
    }
}

/// Add a task to the ready list.
pub fn ready_list_add_task(task: *mut TaskInfo) {
    let task_ref = unsafe { &mut *task };
    task_ref.set_status(TaskStatus::Ready);

    // Return if task is idle task.
    let cpu_idle = unsafe { TASK_LIST.cpu_idle.get() };
    if cpu_idle == task {
        return;
    }

    list::tail_append(unsafe { &mut TASK_LIST.ready_head }, &mut task_ref.list);
}


struct TaskList {
    pub ready_head: List,
    /// Idle task struct on per-cpu.
    pub cpu_idle: PerCpuPtr<TaskInfo>,
}

impl TaskList {
    pub const fn new() -> Self {
        Self {
            ready_head: List::new(),
            cpu_idle: PerCpuPtr::new_empty()
        }
    }
}

static mut TASK_LIST: TaskList = TaskList::new();
