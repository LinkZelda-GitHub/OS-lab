use alloc::{
    collections::*,
    ffi::IntoStringError,
    format,
    sync::{Arc, Weak},
};

// use core::f32::math::fract;;
use boot::entry_point;
use hashbrown::HashMap;
use log::set_logger;
use spin::{Mutex, RwLock};
use uefi::proto::pi;
use x86::current;
use x86_64::structures::paging::page;
use xmas_elf::ElfFile;

use super::*;
use crate::{
    humanized_size,
    memory::{
        self, PAGE_SIZE,
        allocator::{ALLOCATOR, HEAP_SIZE},
        get_frame_alloc_for_sure,
    },
    proc::{self, vm::stack},
};

pub static PROCESS_MANAGER: spin::Once<ProcessManager> = spin::Once::new();

pub fn init(init: Arc<Process>, app_list: boot::AppListRef) {
    // FIXED: set init process as Running
    {
        let mut init_inner = init.write();
        init_inner.resume();
    } // drop the lock before setting processor's pid

    // FIXED: set processor's current pid to init's pid
    processor::set_pid(init.pid());

    PROCESS_MANAGER.call_once(|| ProcessManager::new(init, app_list));
}

use x86_64::instructions::interrupts;

pub fn fork(context: &mut ProcessContext) {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let manager = get_process_manager();
        // FIXED: save_current as parent
        manager.save_current(context);
        // FIXED: fork to get child
        manager.fork();
        // FIXED: push to child & parent to ready queue
        // FIXED: switch to next process
        manager.switch_next(context);
    })
}

pub fn get_process_manager() -> &'static ProcessManager {
    PROCESS_MANAGER
        .get()
        .expect("Process Manager has not been initialized")
}

pub struct ProcessManager {
    processes: RwLock<HashMap<ProcessId, Arc<Process>, ahash::RandomState>>,
    ready_queue: Mutex<VecDeque<ProcessId>>,
    app_list: boot::AppListRef,
    wait_queue: Mutex<HashMap<ProcessId, BTreeSet<ProcessId>, ahash::RandomState>>,
}

impl ProcessManager {
    pub fn new(init: Arc<Process>, app_list: boot::AppListRef) -> Self {
        let mut processes = HashMap::default();
        let ready_queue = VecDeque::new();
        let pid = init.pid();

        trace!("Init {:#?}", init);

        processes.insert(pid, init);
        Self {
            processes: RwLock::new(processes),
            ready_queue: Mutex::new(ready_queue),
            app_list,
            wait_queue: Mutex::new(HashMap::default()),
        }
    }

    #[inline]
    pub fn app_list(&self) -> boot::AppListRef {
        self.app_list
    }

    #[inline]
    pub fn push_ready(&self, pid: ProcessId) {
        self.ready_queue.lock().push_back(pid);
    }

    #[inline]
    pub(super) fn add_proc(&self, pid: ProcessId, proc: Arc<Process>) {
        self.processes.write().insert(pid, proc);
    }

    #[inline]
    pub(super) fn get_proc(&self, pid: &ProcessId) -> Option<Arc<Process>> {
        self.processes.read().get(pid).cloned()
    }

    pub fn current(&self) -> Arc<Process> {
        self.get_proc(&processor::get_pid())
            .expect("No current process")
    }

    pub fn save_current(&self, context: &ProcessContext) {
        let proc = self.current();
        let pid = proc.pid();

        let mut proc_inner = proc.write();
        let was_dead = proc_inner.status() == ProgramStatus::Dead;
        // FIXED: update current process's tick count
        proc_inner.tick();
        // FIXED: save current process's context
        proc_inner.save(context);

        // drop(proc_inner);

        if !was_dead {
            // 特判是不是 Dead 状态，Dead 状态不需要 push 回 ready_queue
            proc_inner.pause();

            drop(proc_inner);

            self.push_ready(pid);
        }
    }

    pub fn switch_next(&self, context: &mut ProcessContext) -> ProcessId {
        loop {
            // FIXED: fetch the next process from ready queue
            let next_pid = {
                let mut ready_queue = self.ready_queue.lock();
                ready_queue.pop_front().unwrap_or(KERNEL_PID)
            };

            // FIXED: check if the next process is ready,
            //        continue to fetch if not ready
            let Some(next_proc) = self.get_proc(&next_pid) else {
                continue;
            };

            let mut next_inner = next_proc.write();
            if !next_inner.is_ready() && next_pid != KERNEL_PID {
                continue;
            }

            // FIXED: restore next process's context
            next_inner.restore(context);

            drop(next_inner);

            // FIXED: update processor's current pid
            processor::set_pid(next_pid);

            // FIXED: return next process's pid
            return next_pid;
        }
    }

    // pub fn spawn_kernel_thread(
    //     &self,
    //     entry: VirtAddr,
    //     name: String,
    //     proc_data: Option<ProcessData>,
    // ) -> ProcessId {
    //     let kproc = self.get_proc(&KERNEL_PID).unwrap();
    //     let page_table = kproc.read().clone_page_table();
    //     let proc_vm = Some(ProcessVm::new(page_table));
    //     let proc = Process::new(name, Some(Arc::downgrade(&kproc)), proc_vm,
    // proc_data);

    //     let pid = proc.pid();

    //     // alloc stack for the new process base on pid
    //     let stack_top = proc.alloc_init_stack();

    //     // FIXED: set the stack frame
    //     proc.write().init_stack_frame(entry, stack_top);

    //     // FIXED: add to process map
    //     self.add_proc(pid, proc);

    //     // FIXED: push to ready queue
    //     self.push_ready(pid);

    //     // FIXED: return new process pid
    //     pid
    // }

    pub fn spawn(
        &self,
        elf: &ElfFile,
        name: String,
        parent: Option<Weak<Process>>,
        proc_data: Option<ProcessData>,
    ) -> ProcessId {
        let kproc = self.get_proc(&KERNEL_PID).unwrap();
        let page_table = kproc.read().clone_page_table();
        let proc_vm = Some(ProcessVm::new(page_table));
        let proc = Process::new(name, parent, proc_vm, proc_data);

        let mut inner = proc.write();
        // FIXED: load elf to process pagetable
        let text_pages = inner.load_elf(elf);
        let stack_pages = stack::STACK_DEF_PAGE as usize;
        let total_memory_bytes = (text_pages + stack_pages) * PAGE_SIZE as usize;
        inner.set_env("ELF_PAGES", &alloc::format!("{}", text_pages));
        inner.set_env("MEM_USAGE", &alloc::format!("{}", total_memory_bytes));

        // FIXED: alloc new stack for process
        let pid = proc.pid();
        let stack_top = inner.vm_mut().init_proc_stack(pid);
        let entry_point = VirtAddr::new(elf.header.pt2.entry_point() as usize as u64);
        inner.init_stack_frame(entry_point, stack_top);

        // FIXED: mark process as ready
        inner.pause();

        drop(inner);

        trace!("New {:#?}", &proc);

        // FIXED: something like kernel thread
        self.add_proc(pid, proc.clone());
        self.push_ready(pid);

        pid
    }

    pub fn kill_current(&self, ret: isize) {
        self.kill(processor::get_pid(), ret);
    }

    pub fn get_exit_code(&self, pid: &ProcessId) -> Option<isize> {
        let proc = self.get_proc(pid)?;
        let inner = proc.read();
        if inner.status() == ProgramStatus::Dead {
            return inner.exit_code();
        } else {
            return None;
        }
    }

    pub fn handle_page_fault(&self, addr: VirtAddr, err_code: PageFaultErrorCode) -> bool {
        // FIXED: handle page fault
        if err_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION) {
            warn!(
                "Page fault at {:#x} with protection violation.",
                addr.as_u64()
            );
            return false;
        }

        let pid = processor::get_pid();
        if let Some(proc) = self.get_proc(&pid) {
            return proc.write().handle_page_fault(addr);
        }
        false
    }

    pub fn kill(&self, pid: ProcessId, ret: isize) {
        let proc = self.get_proc(&pid);

        if proc.is_none() {
            warn!("Process #{} not found.", pid);
            return;
        }

        let proc = proc.unwrap();

        if proc.read().status() == ProgramStatus::Dead {
            warn!("Process #{} is already dead.", pid);
            return;
        }

        trace!("Kill {:#?}", &proc);

        proc.kill(ret);

        if let Some(pids) = self.wait_queue.lock().remove(&pid) {
            for pid in pids {
                self.wake_up(pid, Some(ret));
            }
        }
    }

    pub fn print_process_list(&self) {
        let mut output =
            String::from("  PID | PPID | Process Name |  Ticks  |   Memory  | Status\n");

        self.processes
            .read()
            .values()
            .filter(|p| p.read().status() != ProgramStatus::Dead)
            .for_each(|p| output += format!("{}\n", p).as_str());

        // FIXED: print memory usage of kernel heap
        let alloc = get_frame_alloc_for_sure();
        let frames_used = alloc.frames_used();
        let frames_total = alloc.frames_total();

        let used = frames_used * PAGE_SIZE as usize;
        let total = frames_total * PAGE_SIZE as usize;

        output += &format_usage("Memory", used, total);
        drop(alloc);

        output += format!("Queue  : {:?}\n", self.ready_queue.lock()).as_str();

        output += &processor::print_processors();

        print!("{}", output);
    }

    pub fn fork(&self) {
        // FIXED: get current process
        let current = self.current();
        // FIXED: fork to get child
        let child = current.fork();
        let child_pid = child.pid();
        // FIXED: add child to process list
        self.add_proc(child_pid, child.clone());
        self.push_ready(child_pid);
        // FOR DBG: maybe print the process ready queue?
        log::trace!(
            "Process {:?} successfully forked child {:?}",
            current.pid(),
            child_pid
        );
    }

    /// Block the process with the given pid
    pub fn block(&self, pid: ProcessId) {
        if let Some(proc) = self.get_proc(&pid) {
            // FIXED: set the process as blocked
            let mut proc_inner = proc.write();
            proc_inner.block();
        }
    }

    pub fn wait_pid(&self, pid: ProcessId) {
        let mut wait_queue = self.wait_queue.lock();
        // FIXED: push the current process to the wait queue
        //        `processor::get_pid()` is waiting for `pid`
        let entry = wait_queue.entry(pid).or_default();
        entry.insert(processor::get_pid());
    }

    /// Wake up the process with the given pid
    ///
    /// If `ret` is `Some`, set the return value of the process
    pub fn wake_up(&self, pid: ProcessId, ret: Option<isize>) {
        if let Some(proc) = self.get_proc(&pid) {
            let mut inner = proc.write();
            if let Some(ret) = ret {
                // FIXED: set the return value of the process
                //        like `context.set_rax(ret as usize)`
                inner.set_rax(ret as usize);
            }
            // FIXED: set the process as ready
            inner.pause();
            drop(inner);
            // FIXED: push to ready queue
            self.push_ready(pid);
        }
    }
}

// A helper function to format memory usage
fn format_usage(name: &str, used: usize, total: usize) -> String {
    let (used_float, used_unit) = humanized_size(used as u64);
    let (total_float, total_unit) = humanized_size(total as u64);

    format!(
        "{:<6} : {:>6.*} {:>3} / {:>6.*} {:>3} ({:>5.2}%)\n",
        name,
        2,
        used_float,
        used_unit,
        2,
        total_float,
        total_unit,
        used as f32 / total as f32 * 100.0
    )
}
