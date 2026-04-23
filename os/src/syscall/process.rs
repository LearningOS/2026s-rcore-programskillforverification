//! Process management syscalls
use crate::config::PAGE_SIZE;
use crate::mm::{translated_byte_buffer, MapPermission, VirtAddr};
use crate::task::{
    change_program_brk, current_user_token, exit_current_and_run_next, get_syscall_times,
    mmap_current, munmap_current, read_user_byte_current, suspend_current_and_run_next,
    write_user_byte_current,
};
use crate::timer::get_time_us;

/// Trace request: read one byte of user memory.
const TRACE_READ: usize = 0;
/// Trace request: write one byte of user memory.
const TRACE_WRITE: usize = 1;
/// Trace request: query current task's invocation count for a syscall id.
const TRACE_SYSCALL: usize = 2;

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
    let us = get_time_us();
    let tv = TimeVal {
        sec: us / 1_000_000,
        usec: us % 1_000_000,
    };
    let buffers = translated_byte_buffer(
        current_user_token(),
        ts as *const u8,
        core::mem::size_of::<TimeVal>(),
    );
    let src = unsafe {
        core::slice::from_raw_parts(
            &tv as *const TimeVal as *const u8,
            core::mem::size_of::<TimeVal>(),
        )
    };
    let mut offset = 0;
    for buffer in buffers {
        let len = buffer.len();
        buffer.copy_from_slice(&src[offset..offset + len]);
        offset += len;
    }
    0
}

/// sys_trace dispatches read/write of a user byte, or a syscall-count query.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    match trace_request {
        TRACE_READ => match read_user_byte_current(VirtAddr::from(id)) {
            Some(byte) => byte as isize,
            None => -1,
        },
        TRACE_WRITE => {
            if data > u8::MAX as usize {
                return -1;
            }
            if write_user_byte_current(VirtAddr::from(id), data as u8) {
                0
            } else {
                -1
            }
        }
        TRACE_SYSCALL => get_syscall_times(id) as isize,
        _ => -1,
    }
}

/// sys_mmap maps `[start, start+len)` with the given port (bits: R=1, W=2, X=4).
pub fn sys_mmap(start: usize, len: usize, port: usize) -> isize {
    trace!("kernel: sys_mmap");
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    if port & !0x7 != 0 || port & 0x7 == 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    let mut perm = MapPermission::U;
    if port & 0x1 != 0 {
        perm |= MapPermission::R;
    }
    if port & 0x2 != 0 {
        perm |= MapPermission::W;
    }
    if port & 0x4 != 0 {
        perm |= MapPermission::X;
    }
    mmap_current(VirtAddr::from(start), VirtAddr::from(start + len), perm)
}

/// sys_munmap unmaps `[start, start+len)`, fails if any page isn't mapped.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap");
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    munmap_current(VirtAddr::from(start), VirtAddr::from(start + len))
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
