//! Process management syscalls

use crate::mm::{translated_byte_buffer, PTEFlags, PageTable, VirtAddr};
use crate::task::{change_program_brk, current_user_token, exit_current_and_run_next, fetch_syscall_count, suspend_current_and_run_next};
use crate::timer;
use core::mem::size_of;

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
    let us = timer::get_time_us();
    trace!("kernel: sys_get_time {}", us);

    const U: usize = 1_000_000usize;

    let mut tv_idx = 0;
    let tv = &TimeVal {
        sec: us / U,
        usec: us % U,
    } as *const TimeVal as *const u8;
    let tv_size = size_of::<TimeVal>();
    let bufs = translated_byte_buffer(current_user_token(), _ts as *const u8, tv_size);

    for buf in bufs {
        for i in 0..buf.len() {
            unsafe { buf[i] = *tv.add(tv_idx) }
            tv_idx += 1;
        }
    }

    0
}

/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");

    let internal = |flag|{
        let satp = current_user_token();
        let pg = PageTable::from_token(satp);
        let vpn = VirtAddr::from(_id).floor();

        if let Some(pte) =  pg.translate(vpn){
            if !pte.is_valid() || ((pte.flags() & flag) == PTEFlags::empty()) {
                return -1;
            }
        }else{
            return -1;
        }


        let mut bufs = translated_byte_buffer(satp,_id as *const u8,1);
        let a = &mut bufs[0];

        match flag {
            PTEFlags::R =>  bufs[0][0] as isize,
            PTEFlags::W => {
                a[0] = _data as u8;
                0
            }
            _ => panic!("Invalid flag!"),
        }
    };

    match _trace_request {
        2 => fetch_syscall_count(_id),
        1 => internal(PTEFlags::W),
        0 => internal(PTEFlags::R),
        _ => panic!("Unsupported sys_trace request: {}", _trace_request),
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    -1
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    -1
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
