use alloc::{
    collections::*,
    ffi::IntoStringError,
    format,
    sync::{Arc, Weak},
};

use boot::entry_point;
use hashbrown::HashMap;
use spin::{Mutex, RwLock};
use uefi::proto::pi;
use x86_64::structures::paging::page;
use xmas_elf::ElfFile;

use super::*;
use crate::{
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

pub fn get_process_manager() -> &'static ProcessManager {
    PROCESS_MANAGER
        .get()
        .expect("Process Manager has not been initialized")
}

pub struct ProcessManager {
    processes: RwLock<HashMap<ProcessId, Arc<Process>, ahash::RandomState>>,
    ready_queue: Mutex<VecDeque<ProcessId>>,
    app_list: boot::AppListRef,
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
    fn add_proc(&self, pid: ProcessId, proc: Arc<Process>) {
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
    }

    pub fn print_process_list(&self) {
        let mut output =
            String::from("  PID | PPID | Process Name |  Ticks  | Status | Memory Usage\n");

        self.processes
            .read()
            .values()
            .filter(|p| p.read().status() != ProgramStatus::Dead)
            .for_each(|p| output += format!("{}\n", p).as_str());

        // TODO: print memory usage of kernel heap

        output += format!("Queue  : {:?}\n", self.ready_queue.lock()).as_str();

        output += &processor::print_processors();

        print!("{}", output);
    }
}
