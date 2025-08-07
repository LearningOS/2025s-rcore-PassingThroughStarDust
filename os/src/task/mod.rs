//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the operating system.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

mod context;
mod switch;
#[allow(clippy::module_inception)]
mod task;

use crate::loader::{get_app_data, get_num_app};
use crate::sync::UPSafeCell;
use crate::trap::TrapContext;
use alloc::vec::Vec;
use lazy_static::*;
use switch::__switch;
pub use task::{TaskControlBlock, TaskStatus};

pub use context::TaskContext;
// ** for chapter 4 exercises
use crate::{
    mm::VirtAddr,
    syscall::{
        SYSCALL_WRITE,
        SYSCALL_EXIT,
        SYSCALL_YIELD,
        SYSCALL_GET_TIME,
        SYSCALL_TRACE,
        SYSCALL_MMAP,
        SYSCALL_MUNMAP,
        SYSCALL_SBRK
    }
};

/// The task manager, where all the tasks are managed.
///
/// Functions implemented on `TaskManager` deals with all task state transitions
/// and task context switching. For convenience, you can find wrappers around it
/// in the module level.
///
/// Most of `TaskManager` are hidden behind the field `inner`, to defer
/// borrowing checks to runtime. You can see examples on how to use `inner` in
/// existing functions on `TaskManager`.
pub struct TaskManager {
    /// total number of tasks
    num_app: usize,
    /// use inner value to get mutable access
    inner: UPSafeCell<TaskManagerInner>,
}

/// The task manager inner in 'UPSafeCell'
struct TaskManagerInner {
    /// task list
    tasks: Vec<TaskControlBlock>,
    /// id of current `Running` task
    current_task: usize,
}

lazy_static! {
    /// a `TaskManager` global instance through lazy_static!
    pub static ref TASK_MANAGER: TaskManager = {
        println!("init TASK_MANAGER");
        let num_app = get_num_app();
        println!("num_app = {}", num_app);
        let mut tasks: Vec<TaskControlBlock> = Vec::new();
        for i in 0..num_app {
            tasks.push(TaskControlBlock::new(get_app_data(i), i));
        }
        TaskManager {
            num_app,
            inner: unsafe {
                UPSafeCell::new(TaskManagerInner {
                    tasks,
                    current_task: 0,
                })
            },
        }
    };
}

impl TaskManager {
    /// Run the first task in task list.
    ///
    /// Generally, the first task in task list is an idle task (we call it zero process later).
    /// But in ch4, we load apps statically, so the first task is a real app.
    fn run_first_task(&self) -> ! {
        let mut inner = self.inner.exclusive_access();
        let next_task = &mut inner.tasks[0];
        next_task.task_status = TaskStatus::Running;
        let next_task_cx_ptr = &next_task.task_cx as *const TaskContext;
        drop(inner);
        let mut _unused = TaskContext::zero_init();
        // before this, we should drop local variables that must be dropped manually
        unsafe {
            __switch(&mut _unused as *mut _, next_task_cx_ptr);
        }
        panic!("unreachable in run_first_task!");
    }

    /// Change the status of current `Running` task into `Ready`.
    fn mark_current_suspended(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Ready;
    }

    /// Change the status of current `Running` task into `Exited`.
    fn mark_current_exited(&self) {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].task_status = TaskStatus::Exited;
    }

    /// Find next task to run and return task id.
    ///
    /// In this case, we only return the first `Ready` task in task list.
    fn find_next_task(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        let current = inner.current_task;
        (current + 1..current + self.num_app + 1)
            .map(|id| id % self.num_app)
            .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
    }

    /// Get the current 'Running' task's token.
    fn get_current_token(&self) -> usize {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_user_token()
    }

    /// Get the current 'Running' task's trap contexts.
    fn get_current_trap_cx(&self) -> &'static mut TrapContext {
        let inner = self.inner.exclusive_access();
        inner.tasks[inner.current_task].get_trap_cx()
    }

    /// Change the current 'Running' task's program break
    pub fn change_current_program_brk(&self, size: i32) -> Option<usize> {
        let mut inner = self.inner.exclusive_access();
        let cur = inner.current_task;
        inner.tasks[cur].change_program_brk(size)
    }

    /// Switch current `Running` task to the task we have found,
    /// or there is no `Ready` task and we can exit with all applications completed
    fn run_next_task(&self) {
        if let Some(next) = self.find_next_task() {
            let mut inner = self.inner.exclusive_access();
            let current = inner.current_task;
            inner.tasks[next].task_status = TaskStatus::Running;
            inner.current_task = next;
            let current_task_cx_ptr = &mut inner.tasks[current].task_cx as *mut TaskContext;
            let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
            drop(inner);
            // before this, we should drop local variables that must be dropped manually
            unsafe {
                __switch(current_task_cx_ptr, next_task_cx_ptr);
            }
            // go back to user mode
        } else {
            panic!("All applications completed!");
        }
    }

    // ** for chapter 4 exercises
    /// copy data from kernel space to user space
    pub fn copy_to_user(&self, start_va: VirtAddr, len: usize, buf: &[u8]) {
        let mut inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        let memory_set = &mut inner.tasks[current_task].memory_set;
        memory_set.copy_to_user(start_va, len, buf);
    }

    // ** for chapter 4 exercises
    /// read a byte from the given virtual address in the user space
    pub fn read_byte_from_user(&self, va: VirtAddr) -> isize {
        let mut inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        let memory_set = &mut inner.tasks[current_task].memory_set;
        memory_set.read_byte_from_user(va)
    }

    // ** for chapter 4 exercises
    /// write a given byte to the given virtual address in the user space 
    pub fn write_byte_to_user(&self, va: VirtAddr, data: u8) -> isize {
        let mut inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        let memory_set = &mut inner.tasks[current_task].memory_set;
        memory_set.write_byte_to_user(va, data)
    }

    // ** for chapter 4 exercises
    // get the index of a syscall in the syscall count manager
    /* 
        8 syscall types and their indexes: 
            SYSCALL_WRITE       -       0
            SYSCALL_EXIT        -       1
            SYSCALL_YIELD       -       2
            SYSCALL_GET_TIME    -       3
            SYSCALL_TRACE       -       4
            SYSCALL_MMAP        -       5
            SYSCALL_MUNMAP      -       6
            SYSCALL_SBRK        -       7
    */
    fn get_syscall_count_idx(syscall_id: usize) -> usize {
        match syscall_id {
            SYSCALL_WRITE => 0,
            SYSCALL_EXIT => 1,
            SYSCALL_YIELD => 2,
            SYSCALL_GET_TIME => 3,
            SYSCALL_TRACE => 4,
            SYSCALL_MMAP => 5,
            SYSCALL_MUNMAP => 6,
            SYSCALL_SBRK => 7,
            _ => panic!("Unsupported syscall_id: {}", syscall_id),
        }
    }

    // ** for chapter 4 exercises
    /// add 1 to the syscall count of given syscall id in the user space
    pub fn add_syscall_count(&self, syscall_id: usize) {
        let mut inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        let task = &mut inner.tasks[current_task];
        let idx = Self::get_syscall_count_idx(syscall_id);
        task.syscall_counts[idx] += 1;
    }

    // ** for chapter 4 exercises
    /// get the syscall count of given syscall id in the user space
    pub fn get_syscall_count(&self, syscall_id: usize) -> usize {
        let inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        let task = &inner.tasks[current_task];
        let idx = Self::get_syscall_count_idx(syscall_id);
        task.syscall_counts[idx]
    }

    // ** for chapter 4 exercises
    /// map a range of memory in the user space
    pub fn mmap_to_user(&self, start: VirtAddr, len: usize, port: usize) -> isize {
        let mut inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        let task = &mut inner.tasks[current_task];
        let memory_set = &mut task.memory_set;
        memory_set.mmap_to_user(start, len, port)
    }

    // ** for chapter 4 exercises
    /// unmap a range of memory in the user space
    pub fn munmap_to_user(&self, start: VirtAddr, len: usize) -> isize {
        let mut inner = self.inner.exclusive_access();
        let current_task = inner.current_task;
        let task = &mut inner.tasks[current_task];
        let memory_set = &mut task.memory_set;
        memory_set.munmap_to_user(start, len)
    }
}

/// Run the first task in task list.
pub fn run_first_task() {
    TASK_MANAGER.run_first_task();
}

/// Switch current `Running` task to the task we have found,
/// or there is no `Ready` task and we can exit with all applications completed
fn run_next_task() {
    TASK_MANAGER.run_next_task();
}

/// Change the status of current `Running` task into `Ready`.
fn mark_current_suspended() {
    TASK_MANAGER.mark_current_suspended();
}

/// Change the status of current `Running` task into `Exited`.
fn mark_current_exited() {
    TASK_MANAGER.mark_current_exited();
}

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    mark_current_suspended();
    run_next_task();
}

/// Exit the current 'Running' task and run the next task in task list.
pub fn exit_current_and_run_next() {
    mark_current_exited();
    run_next_task();
}

/// Get the current 'Running' task's token.
pub fn current_user_token() -> usize {
    TASK_MANAGER.get_current_token()
}

/// Get the current 'Running' task's trap contexts.
pub fn current_trap_cx() -> &'static mut TrapContext {
    TASK_MANAGER.get_current_trap_cx()
}

/// Change the current 'Running' task's program break
pub fn change_program_brk(size: i32) -> Option<usize> {
    TASK_MANAGER.change_current_program_brk(size)
}

// ** for chapter 4 exercises
/// copy data from kernel space to the current user space
pub fn copy_to_user(start_va: VirtAddr, len: usize, buf: &[u8]) {
    TASK_MANAGER.copy_to_user(start_va, len, buf);
}

// ** for chapter 4 exercises
/// read a byte from the given virtual address in the user space
pub fn read_byte_from_user(va: VirtAddr) -> isize {
    TASK_MANAGER.read_byte_from_user(va)
}

// ** for chapter 4 exercises
/// write a given byte to the given virtual address in the user space 
pub fn write_byte_to_user(va: VirtAddr, data: u8) -> isize {
    TASK_MANAGER.write_byte_to_user(va, data)
}

// ** for chapter 4 exercises
/// add 1 to the syscall count of given syscall id in the user space
pub fn add_syscall_count(syscall_id: usize) {
    TASK_MANAGER.add_syscall_count(syscall_id);
}

// ** for chapter 4 exercises
/// get the syscall count of given syscall id in the user space
pub fn get_syscall_count(syscall_id: usize) -> usize {
    TASK_MANAGER.get_syscall_count(syscall_id)
}

// ** for chapter 4 exercises
/// map a range of memory in the user space
pub fn mmap_to_user(start: VirtAddr, len: usize, port: usize) -> isize {
    TASK_MANAGER.mmap_to_user(start, len, port)
}

// ** for chapter 4 exercises
/// unmap a range of memory in the user space
pub fn munmap_to_user(start: VirtAddr, len: usize) -> isize {
    TASK_MANAGER.munmap_to_user(start, len)
}