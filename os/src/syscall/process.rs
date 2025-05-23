//! Process management syscalls
use crate::task::{change_program_brk, exit_current_and_run_next, suspend_current_and_run_next};

// ** for chapter 4 exerciese
use crate::{
    mm::{
        VirtAddr, MapPermission
    },
    task::{
        copy_from_user,
        copy_to_user,
        get_syscall_count,
        range_map,
        range_unmap
    }, 
    timer::get_time_us,
    config::MAX_SYSCALL_NUM
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
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    // -1
    let us = get_time_us();
    let res =  TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    let ptr = &res as *const TimeVal as *const u8;
    let len = core::mem::size_of::<TimeVal>();
    let buf = unsafe{core::slice::from_raw_parts(ptr, len)};
    let va: VirtAddr = (_ts as usize).into();
    copy_to_user(va, len, buf);
    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    // -1

    match _trace_request {
        0 => {
            let mut res: u8 = 0;
            let len = core::mem::size_of::<u8>();
            let ptr = &mut res as * mut u8;
            let buf = unsafe{core::slice::from_raw_parts_mut(ptr, len)};
            if copy_from_user(_id.into(), len, buf) >= 0 {
                res as isize
            } else {
                -1
            }
        },
        1 => {
            let len = core::mem::size_of::<u8>();
            let ptr = &_data as *const usize as * const u8;
            let buf = unsafe{core::slice::from_raw_parts(ptr, len)};
            if copy_to_user(_id.into(), len, buf) >= 0 {
                0
            } else {
                -1
            }
        },
        2 => {
            if _id >= MAX_SYSCALL_NUM {
                -1
            } else {
                get_syscall_count(_id) as isize
            }
        },
        _ => {
            -1
        },
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    // -1
    let start_va: VirtAddr = _start.into();
    let end_va: VirtAddr = (start_va.0 + _len).into();
    if !start_va.aligned() || _port & !0x7 != 0 || _port & 0x7 == 0 {
        return -1;
    }
    let start_vpn = start_va.floor();
    let end_vpn = end_va.ceil();
    let mut map_perm = MapPermission::U;
    if _port & 1 != 0 {
        map_perm = map_perm | MapPermission::R;
    }
    if (_port >> 1) & 1 != 0 {
        map_perm = map_perm | MapPermission::W;
    }
    if (_port >> 2) & 1 != 0 {
        map_perm = map_perm | MapPermission::X;
    }
    range_map(start_vpn, end_vpn, map_perm)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    // -1
    let start_va: VirtAddr = _start.into();
    let end_va: VirtAddr = (start_va.0 + _len).into();
    if !start_va.aligned() {
        return -1;
    }
    let start_vpn = start_va.floor();
    let end_vpn = end_va.ceil();
    range_unmap(start_vpn, end_vpn)
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
