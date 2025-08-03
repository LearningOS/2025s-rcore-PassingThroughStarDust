//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next},
    timer::get_time_us,
};
// ** for chapter 3 exercises
use crate::task::get_syscall_count;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// TODO: implement the syscall
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    // -1
    match trace_request {
        // "id" as *const u8 to read a byte of unsigned integer
        0 => unsafe { (id as *const u8).read_volatile() as isize },

        // "id" as *mut u8 to write the lowest byte of "data"
        1 => {
            /*
                Ways to get the lowest bit of an usize number(use uz as the source data):
                1. (uz & 0xff) as u8
                2. uz as u8
                3. uz.to_le_bytes()[0]
            */
            unsafe { (id as *mut u8).write_volatile(data as u8); }
            0
        },  

        // "id" as task number to check the count of 
        // the current task calling a system call of syscall id "id"
        2 => get_syscall_count(id) as isize,

        // otherwise return -1
        _ => -1,
    }
}
