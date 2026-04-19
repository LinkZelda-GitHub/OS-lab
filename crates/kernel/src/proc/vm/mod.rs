use alloc::format;

use elf::unmap_range;
use x86_64::{
    VirtAddr,
    structures::paging::{page::*, *},
};
use xmas_elf::ElfFile;

use crate::{humanized_size, memory::*};

pub mod stack;

use self::stack::*;
use super::{PageTableContext, ProcessId};

type MapperRef<'a> = &'a mut OffsetPageTable<'static>;
type FrameAllocatorRef<'a> = &'a mut BootInfoFrameAllocator;

pub struct ProcessVm {
    // page table is shared by parent and child
    pub(super) page_table: PageTableContext,

    // stack is pre-process allocated
    pub(super) stack: Stack,
}

impl ProcessVm {
    pub fn new(page_table: PageTableContext) -> Self {
        Self {
            page_table,
            stack: Stack::empty(),
        }
    }

    #[inline]
    pub fn clone_page_table(&self) -> PageTableContext {
        // self.page_table.clone()
        // PageTableContext::new()
        self.page_table.clone_level_4()
    }

    pub fn alloc_init_stack(&mut self, pid: ProcessId) -> VirtAddr {
        let mut mapper = self.page_table.mapper();
        let stack_top_addr = self.stack.alloc_init_stack(pid, &mut mapper);
        stack_top_addr
    }

    pub fn init_kernel_vm(mut self) -> Self {
        // TODO: record kernel code usage
        self.stack = Stack::kstack();
        self
    }

    pub fn init_proc_stack(&mut self, pid: ProcessId) -> VirtAddr {
        // FIXED: calculate the stack for pid
        // FIXED: calculate the stack for pid
        self.alloc_init_stack(pid)
    }

    pub fn handle_page_fault(&mut self, addr: VirtAddr) -> bool {
        let mapper = &mut self.page_table.mapper();
        let alloc = &mut *get_frame_alloc_for_sure();

        self.stack.handle_page_fault(addr, mapper, alloc)
    }

    pub(super) fn memory_usage(&self) -> u64 {
        self.stack.memory_usage()
    }

    pub fn load_elf(&mut self, elf: &ElfFile) -> usize {
        let mapper = &mut self.page_table.mapper();
        let alloc = &mut *get_frame_alloc_for_sure();

        let pages = elf::load_elf(
            elf,
            crate::memory::PHYSICAL_OFFSET.get().unwrap().clone(),
            mapper,
            alloc,
            true, // user access
        )
        .expect("Failed to load ELF");

        pages
    }
}

impl Drop for ProcessVm {
    fn drop(&mut self) {
        let mapper = &mut self.page_table.mapper();
        let alloc = &mut *get_frame_alloc_for_sure();

        let stack_start_addr = self.stack.range.start.start_address().as_u64();

        let stack_page_count = self.stack.usage;

        if stack_page_count > 0 && stack_start_addr != 0 {
            trace!("Deallocating Process Stack at {:#x}", stack_start_addr);
            let _ = unmap_range(stack_start_addr, stack_page_count, mapper, alloc);
        }

        // TODO
    }
}

impl core::fmt::Debug for ProcessVm {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        let (size, unit) = humanized_size(self.memory_usage());

        f.debug_struct("ProcessVm")
            .field("stack", &self.stack)
            .field("memory_usage", &format!("{} {}", size, unit))
            .field("page_table", &self.page_table)
            .finish()
    }
}
