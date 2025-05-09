//! Process management syscalls
use crate::mm::{frame_usable_nums, translated_byte_buffer, MapPermission, PageTable, VirtAddr};
use crate::task::{mmap, munmap};
use crate::{
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    },
    timer,
};


use alloc::sync::Arc;
use core::mem::size_of;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!(
        "kernel::pid[{}] sys_waitpid [{}]",
        current_task().unwrap().pid.0,
        pid
    );
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
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

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );

    info!(
        "kernel: sys_mmap, va:[{:x}, +{:x}),_port:{:x}",
        _start, _len, _port
    );

    //check params
    let start_va = VirtAddr::from(_start);

    if !start_va.aligned() || _port & (!0x07) != 0 || _port & 0x07 == 0 {
        error!(
            "sys_mmap:virtual address: {:x} not aligned or invalid permission :{:x}",
            _start, _port
        );
        return -1;
    }

    let start_vpn = start_va.floor();
    let end_va = VirtAddr::from(_start + _len);
    let end_vpn = end_va.floor();

    info!("vpn:[{:x},{:x})", start_vpn.0, end_vpn.0,);

    let satp = current_user_token();
    let pg = PageTable::from_token(satp);

    if let Some(pte) = pg.translate(start_vpn) {
        if pte.is_valid() {
            error!("sys_mmap: start addr already mapped! {:x}", pte.bits);
            return -1;
        }
    }

    if let Some(pte) = pg.translate(end_vpn) {
        if pte.is_valid() {
            error!("sys_mmap: end addr is already mapped! {:x}", pte.bits);
            return -1;
        }
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

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let start_va = VirtAddr::from(_start);
    let end_va = VirtAddr::from(_start + _len);
    info!("kernel: sys_munmap [{},{})", start_va.0, end_va.0);

    if !start_va.aligned() {
        return -1;
    }
    munmap(start_va, end_va)
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let token = current_user_token();
    let path = translated_str(token, _path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        let newtask = task.spawn(data);
        let trap_cx = newtask.inner_exclusive_access().get_trap_cx();
        trap_cx.x[10] = 0;
        let new_task_pid =  newtask.pid.0;
        add_task(newtask);
        new_task_pid as isize
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}
