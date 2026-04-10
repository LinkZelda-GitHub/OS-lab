mod context;
mod data;
mod manager;
mod paging;
mod pid;
mod process;
pub mod processor;
mod vm;

use alloc::string::String;

pub use context::ProcessContext;
pub use data::ProcessData;
use manager::*;
pub use paging::PageTableContext;
pub use pid::ProcessId;
use process::*;
use x86_64::{VirtAddr, structures::idt::PageFaultErrorCode};

use crate::{alloc::string::ToString, memory::PAGE_SIZE};
pub const KERNEL_PID: ProcessId = ProcessId(1);

use vm::ProcessVm;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ProgramStatus {
    Running,
    Ready,
    Blocked,
    Dead,
}

/// init process manager
pub fn init() {
    let proc_vm = ProcessVm::new(PageTableContext::new()).init_kernel_vm();

    trace!("Init kernel vm: {:#?}", proc_vm);

    let mut kdata = ProcessData::new();
    kdata.set_env("PAGE_SIZE", &PAGE_SIZE.to_string());
    kdata.set_env(
        "KERNEL_HEAP_SIZE",
        &crate::memory::allocator::HEAP_SIZE.to_string(),
    );
    kdata.set_env("KERNEL_PID", "1");

    // kernel process
    /* FIXED: create kernel process */
    let kproc = Process::new("kernel".into(), None, Some(proc_vm), Some(kdata));

    manager::init(kproc);

    info!("Process Manager Initialized.");
}

pub fn switch(context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        // FIXED: switch to the next process
        let manager = get_process_manager();
        //      - save current process's context
        manager.save_current(context);
        //      - handle ready queue update
        //      - restore next process's context
        manager.switch_next(context);
    });
}

pub fn get_process_exit_code(pid: ProcessId) -> Option<isize> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().get_exit_code(&pid)
    })
}

pub fn spawn_kernel_thread(entry: fn() -> !, name: String, data: Option<ProcessData>) -> ProcessId {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let entry = VirtAddr::new(entry as usize as u64);
        get_process_manager().spawn_kernel_thread(entry, name, data)
    })
}

pub fn print_process_list() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().print_process_list();
    })
}

pub fn env(key: &str) -> Option<String> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        // FIXED: get current process's environment variable
        get_process_manager().current().read().env(key)
    })
}

pub fn process_exit(ret: isize) -> ! {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().kill_current(ret);
    });

    loop {
        x86_64::instructions::hlt();
    }
}

pub fn handle_page_fault(addr: VirtAddr, err_code: PageFaultErrorCode) -> bool {
    x86_64::instructions::interrupts::without_interrupts(|| {
        get_process_manager().handle_page_fault(addr, err_code)
    })
}
