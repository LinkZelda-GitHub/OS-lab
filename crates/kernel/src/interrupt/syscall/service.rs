use core::alloc::Layout;

use uefi::{boot::exit, proto::device_path::build::messaging::RestServiceVendorData};

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

pub fn wait_pid(args: &SyscallArgs) -> isize {
    let pid = crate::proc::ProcessId(args.arg0 as u16);

    if crate::proc::still_alive(pid) {
        return -1isize; // still alive
    }

    if let Some(code) = crate::proc::get_process_exit_code(pid) {
        code
    } else {
        -2isize // no such process
    }
}

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
