//!Implementation of [`TaskManager`]

use core::cmp::Ordering;

use super::TaskControlBlock;
use crate::config::BIGSTRIDE;   // ** for chapter 5 exercises
use crate::sync::UPSafeCell;
use alloc::collections::binary_heap::BinaryHeap;    // ** for chapter 5 exercises
//use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::*;

pub struct StrideTask {
    task: Arc<TaskControlBlock>,
    pass: usize,
}
impl StrideTask {
    pub fn new(task: Arc<TaskControlBlock>) -> Self {
        let priority = task.inner_exclusive_access().priority;
        let pass = BIGSTRIDE / priority;
        Self {
            task,
            pass,
        }
    }
    pub fn add_stride(&mut self) {
        self.task.inner_exclusive_access().stride += self.pass;
    }
}
impl PartialEq for StrideTask {
    fn eq(&self, other: &Self) -> bool {
        self.task.inner_exclusive_access().stride == other.task.inner_exclusive_access().stride
    }
}
impl Eq for StrideTask {}
impl PartialOrd for StrideTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for StrideTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // 采用min-heap，stride越小优先级越高
        other.task.inner_exclusive_access().stride.cmp(&self.task.inner_exclusive_access().stride)
    }
}

///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    stride_queue: BinaryHeap<StrideTask>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            //ready_queue: VecDeque::new(),
            stride_queue: BinaryHeap::new(),
        }
    }

    // ** for chapter 5 exercises
    /// Add process to stride queue with specified stride
    pub fn add_stride(&mut self, task: Arc<TaskControlBlock>) {
        self.stride_queue.push(StrideTask::new(task));
    }

    // ** for chapter 5 exercises
    /// Take a process with lowest pass value from stride queue
    pub fn fetch_stride(&mut self) -> Option<Arc<TaskControlBlock>> {
        if let Some(mut stride_task) = self.stride_queue.pop() {
            stride_task.add_stride();
            let task = Arc::clone(&stride_task.task);
            Some(task)
        } else {
            None
        }
    }

    /*
        /// Add process back to ready queue
        pub fn add(&mut self, task: Arc<TaskControlBlock>) {
            self.ready_queue.push_back(task);
        }
        /// Take a process out of the ready queue
        pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
            self.ready_queue.pop_front()
        }
    */
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add_stride(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch_stride()
}