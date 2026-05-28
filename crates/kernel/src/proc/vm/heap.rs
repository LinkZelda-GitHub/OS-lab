use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};

use x86::current;
use x86_64::{
    VirtAddr,
    structures::paging::{Page, mapper::UnmapError, page::PageRangeInclusive},
};

use super::{FrameAllocatorRef, MapperRef};

// user process runtime heap
// 0x100000000 bytes -> 4GiB
// from 0x0000_2000_0000_0000 to 0x0000_2000_ffff_fff8
pub const HEAP_START: u64 = 0x2000_0000_0000;
pub const HEAP_PAGES: u64 = 0x100000;
pub const HEAP_SIZE: u64 = HEAP_PAGES * crate::memory::PAGE_SIZE;
pub const HEAP_END: u64 = HEAP_START + HEAP_SIZE - 8;

/// User process runtime heap
///
/// always page aligned, the range is [base, end)
pub struct Heap {
    /// the base address of the heap
    ///
    /// immutable after initialization
    base: VirtAddr,

    /// the current end address of the heap
    ///
    /// use atomic to allow multiple threads to access the heap
    end: Arc<AtomicU64>,
}

impl Heap {
    pub fn empty() -> Self {
        Self {
            base: VirtAddr::new(HEAP_START),
            end: Arc::new(AtomicU64::new(HEAP_START)),
        }
    }

    pub fn fork(&self) -> Self {
        Self {
            base: self.base,
            end: self.end.clone(),
        }
    }

    pub fn brk(
        &self,
        new_end: Option<VirtAddr>,
        mapper: MapperRef,
        alloc: FrameAllocatorRef,
    ) -> Option<VirtAddr> {
        // FIXED: if new_end is None, return the current end address
        let current_end_u64 = self.end.load(Ordering::Relaxed);
        let current_end = VirtAddr::new(current_end_u64);
        let new_end = match new_end {
            Some(addr) => addr,
            None => return Some(current_end),
        };
        // FIXED: check if the new_end is valid (in range [base, base + HEAP_SIZE])
        if new_end < self.base || new_end.as_u64() > HEAP_END {
            return None;
        }
        // FIXED: calculate the difference between the current end and the new end
        let current_end_align = current_end.align_up(crate::memory::PAGE_SIZE).as_u64();
        let new_end_align = new_end.align_up(crate::memory::PAGE_SIZE).as_u64();
        let difference = if new_end_align > current_end_align {
            new_end_align - current_end_align
        } else {
            current_end_align - new_end_align
        };

        // NOTE: print the heap difference for debugging
        log::debug!(
            "heap brk: current_end = {:#x}, new_end = {:#x}, current_end_align = {:#x}, \
             new_end_align = {:#x}, difference = {:#x}",
            current_end_u64,
            new_end.as_u64(),
            current_end_align,
            new_end_align,
            difference
        );

        // FIXED: do the actual mapping or unmapping
        if new_end == self.base {
            if current_end_align > self.base.as_u64() {
                // unmap the heap pages
                let start_page = Page::containing_address(self.base);
                let end_page = Page::containing_address(VirtAddr::new(current_end_align)) - 1;
                let page_range = PageRangeInclusive {
                    start: start_page,
                    end: end_page,
                };
                let _ = elf::unmap_range(page_range, mapper, alloc, true);
            }
        } else if new_end_align < current_end_align {
            // unmap the heap pages
            let start_page = Page::containing_address(VirtAddr::new(new_end_align));
            let end_page = Page::containing_address(VirtAddr::new(current_end_align)) - 1;
            let page_range = PageRangeInclusive {
                start: start_page,
                end: end_page,
            };
            let _ = elf::unmap_range(page_range, mapper, alloc, true);
        } else if new_end_align > current_end_align {
            // map the heap pages
            let page_count = (new_end_align - current_end_align) / crate::memory::PAGE_SIZE;
            if elf::map_range(current_end_align, page_count, mapper, alloc).is_err() {
                return None;
            }
        }
        // FIXED: update the end address
        self.end.store(new_end.as_u64(), Ordering::Relaxed);

        Some(new_end)
    }

    pub(super) fn clean_up(
        &self,
        mapper: MapperRef,
        dealloc: FrameAllocatorRef,
    ) -> Result<(), UnmapError> {
        if self.memory_usage() == 0 {
            return Ok(());
        }

        if Arc::strong_count(&self.end) > 1 {
            return Ok(());
        }

        // FIXED: load the current end address and **reset it to base** (use `swap`)
        let current_end_u64 = self.end.swap(self.base.as_u64(), Ordering::Relaxed);
        // FIXED: unmap the heap pages
        let current_align_top = VirtAddr::new(current_end_u64).align_up(crate::memory::PAGE_SIZE);
        if current_align_top > self.base {
            let start_page = Page::containing_address(self.base);
            let end_page = Page::containing_address(current_align_top) - 1;
            let page_range = PageRangeInclusive {
                start: start_page,
                end: end_page,
            };
            elf::unmap_range(page_range, mapper, dealloc, true)?;
        }

        Ok(())
    }

    pub fn memory_usage(&self) -> u64 {
        self.end.load(Ordering::Relaxed) - self.base.as_u64()
    }
}

impl core::fmt::Debug for Heap {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Heap")
            .field("base", &format_args!("{:#x}", self.base.as_u64()))
            .field(
                "end",
                &format_args!("{:#x}", self.end.load(Ordering::Relaxed)),
            )
            .finish()
    }
}
