// use alloc::collections::btree_map::Cursor;
use core::alloc::Layout;

use storage::FileSystem;
use uefi::{boot::exit, proto::device_path::build::messaging::RestServiceVendorData};
use x86::current;
use x86_64::VirtAddr;

use super::SyscallArgs;
use crate::{proc::*, utils::*};

pub fn spawn_process(args: &SyscallArgs) -> usize {
    // FIXED: get app name by args
    //       - core::str::from_utf8_unchecked
    //       - core::slice::from_raw_parts
    let ptr = args.arg0 as *const u8;
    let len = args.arg1;

    let name_buf = unsafe { core::slice::from_raw_parts(ptr, len) };
    let name = unsafe { core::str::from_utf8_unchecked(name_buf) };
    // FIXED: spawn the process by name
    // FIXED: handle spawn error, return 0 if failed
    // FIXED: return pid as usize
    if let Some(pid) = crate::proc::spawn(name) {
        pid.0 as usize
    } else {
        0
    }
}

// pub fn wait_pid(args: &SyscallArgs) -> isize {
//     let pid = crate::proc::ProcessId(args.arg0 as u16);

//     if crate::proc::still_alive(pid) {
//         return -1isize; // still alive
//     }

//     if let Some(code) = crate::proc::get_process_exit_code(pid) {
//         code
//     } else {
//         -2isize // no such process
//     }
// }

pub fn sys_list_app() {
    crate::proc::list_app();
}

pub fn sys_write(args: &SyscallArgs) -> usize {
    let fd = args.arg0 as u8;
    let ptr = args.arg1 as *const u8;
    let len = args.arg2;

    // FIXED: get buffer and fd by args
    //       - core::slice::from_raw_parts
    let buf = unsafe { core::slice::from_raw_parts(ptr, len) };
    // FIXED: call proc::write -> isize
    let ret = crate::proc::write(fd, buf);
    // FIXED: return the result as usize
    if ret < 0 { 0 } else { ret as usize }
}

pub fn sys_read(args: &SyscallArgs) -> usize {
    // FIXED: just like sys_write

    let fd = args.arg0 as u8;
    let ptr = args.arg1 as *mut u8;
    let len = args.arg2;

    let buf = unsafe { core::slice::from_raw_parts_mut(ptr, len) };
    let ret = crate::proc::read(fd, buf);
    if ret < 0 { 0 } else { ret as usize }
}

pub fn exit_process(args: &SyscallArgs, context: &mut ProcessContext) {
    // FIXED: exit process with retcode
    let retcode = args.arg0 as isize;
    crate::proc::exit(retcode, context);
}

pub fn list_process() {
    // FIXED: list all processes
    crate::proc::print_process_list();
}

pub fn sys_get_time() -> usize {
    let t = uefi::runtime::get_time().expect("Failed to get UEFI time");

    let time_ms = t.second() as usize * 1000
        + t.minute() as usize * 60 * 1000
        + t.hour() as usize * 3600 * 1000
        + (t.nanosecond() / 1_000_000) as usize;

    time_ms
}

pub fn sys_allocate(args: &SyscallArgs) -> usize {
    let layout = unsafe { (args.arg0 as *const Layout).as_ref().unwrap() };

    if layout.size() == 0 {
        return 0;
    }

    let ret = crate::memory::user::USER_ALLOCATOR
        .lock()
        .allocate_first_fit(*layout);

    match ret {
        Ok(ptr) => ptr.as_ptr() as usize,
        Err(_) => 0,
    }
}

pub fn sys_deallocate(args: &SyscallArgs) {
    let layout = unsafe { (args.arg1 as *const Layout).as_ref().unwrap() };

    if args.arg0 == 0 || layout.size() == 0 {
        return;
    }

    let ptr = args.arg0 as *mut u8;

    unsafe {
        crate::memory::user::USER_ALLOCATOR
            .lock()
            .deallocate(core::ptr::NonNull::new_unchecked(ptr), *layout);
    }
}

pub fn new_sem(key: u32, val: usize) -> usize {
    let current = get_process_manager().current();
    let current_inner = current.write();
    let mut sems = current_inner.semaphores.write();
    if sems.insert(key, val) { 0 } else { 1 }
}

pub fn remove_sem(key: u32) -> usize {
    let current_proc = get_process_manager().current();
    let current_inner = current_proc.write();
    let mut sems = current_inner.semaphores.write();
    if sems.remove(key) { 0 } else { 1 }
}

pub fn sem_wait(key: u32, context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        let pid = processor::get_pid();
        let current_proc = manager.current();
        let ret = current_proc.read().semaphores.read().wait(key, pid);
        match ret {
            SemaphoreResult::Ok => context.set_rax(0),
            SemaphoreResult::NotExist => context.set_rax(1),
            SemaphoreResult::Block(pid) => {
                // FIXED: save, block it, then switch to next
                //        use `save_current` and `switch_next`
                debug!("Process {} is blocked", pid);
                manager.save_current(context);
                manager.current().write().block();
                manager.switch_next(context);
            }
            _ => unreachable!(),
        }
    })
}

pub fn sem_signal(key: u32, context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();

        let ret = manager.current().read().semaphores.read().signal(key);
        match ret {
            crate::proc::SemaphoreResult::Ok => context.set_rax(0),
            crate::proc::SemaphoreResult::NotExist => context.set_rax(1),
            crate::proc::SemaphoreResult::WakeUp(woken_pid) => {
                debug!("Process {} is woken up", woken_pid);
                manager.wake_up(woken_pid, None);
                context.set_rax(0);
            }
            _ => unreachable!(),
        }
    })
}

pub fn sys_sem(args: &SyscallArgs, context: &mut ProcessContext) {
    match args.arg0 {
        0 => context.set_rax(new_sem(args.arg1 as u32, args.arg2)),
        1 => context.set_rax(remove_sem(args.arg1 as u32)),
        2 => sem_signal(args.arg1 as u32, context),
        3 => sem_wait(args.arg1 as u32, context),
        _ => context.set_rax(usize::MAX),
    }
}

pub fn sys_list_dir(args: &SyscallArgs) {
    let ptr = args.arg0 as *const u8;
    let len = args.arg1;
    let path = unsafe { core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr, len)) };
    crate::drivers::filesystem::ls(path);
}

pub fn sys_open(args: &SyscallArgs) -> usize {
    let ptr = args.arg0 as *const u8;
    let len = args.arg1;
    let path = unsafe { core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr, len)) };

    let fs = crate::drivers::filesystem::get_rootfs();
    if let Ok(file_handle) = fs.open_file(path) {
        let fd = crate::proc::get_process_manager()
            .current()
            .write() // Process 返回 ProcessData, 获取 write lock
            .open(crate::utils::resource::Resource::File(file_handle));
        fd as usize
    } else {
        !0usize
    }
}

pub fn sys_close(args: &SyscallArgs) -> usize {
    let fd = args.arg0 as u8;
    crate::proc::get_process_manager()
        .current()
        .write()
        .close(fd);
    0
}

pub fn sys_brk(args: &SyscallArgs) -> usize {
    let new_heap_end = if args.arg0 == 0 {
        None
    } else {
        Some(VirtAddr::new(args.arg0 as u64))
    };
    match brk(new_heap_end) {
        Some(new_heap_end) => new_heap_end.as_u64() as usize,
        None => !0,
    }
}
