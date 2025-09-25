//! File and filesystem-related syscalls
use crate::fs::{open_file, OpenFlags, Stat};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};
// ** for chapter 6 exercises
use crate::{
    fs::{linkat, unlinkat},
    mm::VirtAddr,
    task::copy_to_user
};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    /*
        trace!(
            "kernel:pid[{}] sys_fstat NOT IMPLEMENTED",
            current_task().unwrap().pid.0
        );
        -1
    */
    trace!("kernel:pid[{}] sys_fstat", current_task().unwrap().pid.0);
    // specific return values and their meanings are undefined,
    // assume that 0 for successful and -1 for failed
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    // fd must be an existed value
    if fd < inner.fd_table.len() {
        // File must be opened (returning "Some" value when indexing the fd table)
        if let Some(file) = &inner.fd_table[fd] {
            let stat = file.get_stat();
            let ptr = &stat as *const Stat as *const u8;
            let len = core::mem::size_of::<Stat>();
            let buffer = unsafe { core::slice::from_raw_parts(ptr, len) };
            drop(inner);
            copy_to_user(VirtAddr::from(st as usize), len, buffer);
            return 0;
        }
    }
    -1
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(old_name: *const u8, new_name: *const u8) -> isize {
    /*
        trace!(
            "kernel:pid[{}] sys_linkat NOT IMPLEMENTED",
            current_task().unwrap().pid.0
        );
        -1
    */
    trace!("kernel:pid[{}] sys_linkat", current_task().unwrap().pid.0);
    /*
        Standard linkat interface implemented by Rust should be:
            fn linkat(olddirfd: i32, oldpath: *const u8, newdirfd: i32, newpath: *const u8, flags: u32) -> i32
        but for simplicity and compatibility of implementation, the following parameters are treat as constant
        and thus ignored in function implementation:
            1. olddirfd，newdirfd: constant as AT_FDCWD (-100);
            2. flags: constant as 0.

        Also, for simplicity, the case that new file routine is existed is not in consideration.
    */
    
    // get old name and new name from current user space
    let token = current_user_token();
    let old_path = translated_str(token, old_name);
    let new_path = translated_str(token, new_name);
    // old name and new name shouldn't be the same
    if old_path != new_path {
        return linkat(&old_path, &new_path);
    }
    -1
}

/// YOUR JOB: Implement unlinkat.
pub fn sys_unlinkat(name: *const u8) -> isize {
    /*
        trace!(
            "kernel:pid[{}] sys_unlinkat NOT IMPLEMENTED",
            current_task().unwrap().pid.0
        );
        -1
    */
    trace!("kernel:pid[{}] sys_unlinkat", current_task().unwrap().pid.0);
    /*
        Standard unlinkat interface implemented by Rust should be:
            fn unlinkat(dirfd: i32, path: *const u8, flags: u32) -> i32
        but for simplicity and compatibility of implementation, the following parameters are treat as constant
        and thus ignored in function implementation:
            1. dirfd: constant as AT_FDCWD (-100);
            2. flags: constant as 0.

        Consider the case of completely deleting a file 
    */

    let token = current_user_token();
    let path = translated_str(token, name);
    unlinkat(&path)
}
