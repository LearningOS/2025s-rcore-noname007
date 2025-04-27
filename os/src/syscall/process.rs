//! Process management syscalls

use crate::mm::{
    frame_usable_nums, translated_byte_buffer, MapPermission, PTEFlags, PageTable,
    VirtAddr,
};
use crate::task::{change_program_brk, current_user_token, exit_current_and_run_next, fetch_syscall_count, mmap, munmap, suspend_current_and_run_next};
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

    let internal = |flag| {
        let satp = current_user_token();
        let pg = PageTable::from_token(satp);
        let vpn = VirtAddr::from(_id).floor();

        if let Some(pte) = pg.translate(vpn) {
            if !pte.is_valid() || ((pte.flags() & flag) == PTEFlags::empty()) {
                return -1;
            }
        } else {
            return -1;
        }

        let mut bufs = translated_byte_buffer(satp, _id as *const u8, 1);
        let a = &mut bufs[0];

        match flag {
            PTEFlags::R => bufs[0][0] as isize,
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
    trace!(
        "kernel: sys_mmap, start:{}, len:{},_port{:x}",
        _start,
        _len,
        _port
    );

    //check params
    let start_va = VirtAddr::from(_start);

    info!("{:x}{}",_start,start_va.aligned());

    if !start_va.aligned() || _port & (!0x07) != 0 || _port & 0x07 == 0 {
        error!(
            "sys_mmap:virtual address: {:x} not aligned or invalid permission :{:x}",
            _start, _port
        );
        return -1;
    }

    let satp = current_user_token();
    let pg = PageTable::from_token(satp);
    let start_vpn = start_va.floor();

    if let Some(_) = pg.translate(start_vpn) {
        error!("sys_mmap: start addr already mapped!");
        return -1;
    }

    let end_va = VirtAddr::from(_start + _len - 1);
    info!("{:x}{}",_start + _len - 1,end_va.aligned());

    if let Some(_) = pg.translate(end_va.floor()) {
        error!("sys_mmap: end addr is already mapped!");
        return -1;
    }

    let need_frames = end_va.ceil().0 - start_vpn.0;

    if need_frames > frame_usable_nums() {
        error!("physical memory area is not enough!");
        return -1;
    }

    let mut perm = MapPermission::U;

    if _port & 0x1 != 0 {
        perm |= MapPermission::R;
    }
    if _port & 0x2 != 0 {
        perm |= MapPermission::W;
    }
    if _port & 0x4 != 0 {
        perm |= MapPermission::X;
    }

    mmap(start_va, end_va, perm);
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    let start_va = VirtAddr::from(_start);
    let end_va = VirtAddr::from(_start + _len - 1);

    info!("kernel: sys_munmap [{},{}]", start_va.0, end_va.0);

    let satp = current_user_token();
    let pg = PageTable::from_token(satp);
    let start_vpn = start_va.floor();
    if let Some(_) = pg.translate(start_vpn) {
        error!("sys_munmap: start addr already mapped!");
    }

    munmap(start_va, end_va);

    0
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
