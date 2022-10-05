//! Handle scheduler request.

use crate::proc::task::{TaskInfo, TaskStatus};
use crate::util::list;
use crate::util::list::List;


pub(super) fn init_and_set_idle_task() {
    //
    unsafe {
        TASK_LIST.ready_head.init_empty();
        // todo: add idle task to ready_head with the lowest priority.
    }
}

pub(super) fn find_ready_task_or_idle() -> *mut TaskInfo {
    let task_list = unsafe { &TASK_LIST };
    // At least one idle task is in the ready list.
    debug_assert!(!list::is_empty(&task_list.ready_head));

    unsafe {
        // Remove the first ready task from `ready_head`.
        let next = task_list.ready_head.next;
        let task_info = container_of_mut!(next, TaskInfo, list);
        list::delete(&mut *next);

        task_info
    }
}

pub fn ready_list_add_task(task: *mut TaskInfo) {
    let task_ref = unsafe { &mut *task };
    task_ref.set_status(TaskStatus::Ready);
    list::tail_append(unsafe { &mut TASK_LIST.ready_head }, &mut task_ref.list);
}


struct TaskList {
    pub ready_head: List,
}

impl TaskList {
    pub const fn new() -> Self {
        Self {
            ready_head: List::new(),
        }
    }
}

static mut TASK_LIST: TaskList = TaskList::new();
