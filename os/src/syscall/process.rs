//! Process management syscalls
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next};
// ** for chapter 4 exercises
use crate::{
    timer::get_time_us,
    mm::{
        VirtAddr,
    },
    task::{
        copy_to_user,
        read_byte_from_user,
        write_byte_to_user,
        get_syscall_count,
        mmap_to_user,
        munmap_to_user
    }
};
#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    // -1
    let us = get_time_us();
    let ts0 =  TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    let ptr = &ts0 as *const TimeVal as *const u8;
    let len = core::mem::size_of::<TimeVal>();
    let buf = unsafe{ core::slice::from_raw_parts(ptr, len) };
    let start_va: VirtAddr = (ts as usize).into();
    copy_to_user(start_va, len, buf);
    0

    /*
                                        Summary
        sys_get_time must be rewrited after activating paging mode because its
        first parameter _ts is a pointer pointing to a TimeVal variable in the
        user space, but it's in the kernel space where we assign correct value
        to the TimeVal variable through the pointer. The address stored in _ts
        is not the correct address of the TimeVal variable when CPU switching 
        to the kernel space.

        Once thing noticeable in the implementation is that assigning value to
        TimeVal requires using a value "us" from the kernel space. Since using
        variables, calling functions all requires address, it's incorrect if
        we temporarily switch to the user space, assigning "us" to TimeVal and
        then switch back to the user space.

        For the implementation, I've come up with 2 solutions:
            1. Finding the physical frames where all the bits of the TimeVal
               variable locates and copying data onto them.
               -- The adopted answer

            2. Creating another trampoline to transmit data between the kernel
               space and the user space.
               -- Too complicated to implement, and not works when the length
                  of the data to be copied is longer than the trampoline.
    */
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    // -1

    match trace_request {
        0 => read_byte_from_user(VirtAddr::from(id)),
        1 => write_byte_to_user(VirtAddr::from(id), data as u8),
        2 => get_syscall_count(id) as isize,
        _ => -1,
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    // trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    // -1

    trace!("kernel: sys_mmap");
    mmap_to_user(start.into(), len, port)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    // trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    // -1

    trace!("kernel: sys_munmap");
    munmap_to_user(start.into(), len)
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
