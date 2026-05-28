use core::{
    alloc::{GlobalAlloc, Layout},
    sync::atomic::{AtomicUsize, Ordering},
};

use linked_list_allocator::LockedHeap;

use crate::*;

const HEAP_SIZE: usize = 8 * 1024 - 8; // 8 KiB
const HEAP_MAX_SIZE: usize = 8 * 1024 * 1024; // 8 MiB

// #[global_allocator]
// static ALLOCATOR: LockedHeap = LockedHeap::empty();

// pub fn init() {
//     let heap_start = sys_brk(None).unwrap();
//     let heap_end = heap_start + HEAP_SIZE;

//     let ret = sys_brk(Some(heap_end)).expect("Failed to allocate heap");

//     assert!(ret == heap_end, "Failed to allocate heap");

//     unsafe { ALLOCATOR.lock().init(heap_start as *mut u8, HEAP_SIZE) };
// }

#[global_allocator]
static ALLOCATOR: BrkAllocator = BrkAllocator::empty();
struct BrkAllocator {
    allocator: LockedHeap,
    total_size: AtomicUsize,
}

pub fn init() {
    ALLOCATOR.init();
}

impl BrkAllocator {
    pub const fn empty() -> Self {
        Self {
            allocator: LockedHeap::empty(),
            total_size: AtomicUsize::new(0),
        }
    }

    pub fn init(&self) {
        // FIXED: init heap to initial size with `brk` system call
        let heap_start = sys_brk(None).unwrap();
        let heap_end = heap_start + HEAP_SIZE;

        let ret = sys_brk(Some(heap_end)).expect("Failed to allocate heap");

        assert!(ret == heap_end, "Failed to allocate heap");

        unsafe { self.allocator.lock().init(heap_start as *mut u8, HEAP_SIZE) };

        self.total_size.store(HEAP_SIZE, Ordering::Relaxed);
    }

    pub unsafe fn extend(&self, layout: Layout) -> bool {
        // FIXED: extend heap size with `brk` system call
        //        return false if failed or reach the max size (8 MiB suggested)
        let required = layout.size();

        loop {
            // use loop to retry if compare_exchange failed
            let current_size = self.total_size.load(Ordering::SeqCst);
            let new_size = current_size + required;
            if new_size > HEAP_MAX_SIZE {
                return false;
            }
            if self
                .total_size
                .compare_exchange(current_size, new_size, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                let head_start = sys_brk(None).unwrap();
                let head_new_end = head_start + required;
                let ret = sys_brk(Some(head_new_end)).expect("Failed to extend heap");
                if ret == head_new_end {
                    unsafe { self.allocator.lock().extend(required) };
                    return true;
                } else {
                    // rollback the total size if failed
                    self.total_size.fetch_sub(required, Ordering::SeqCst);
                    return false;
                }
            }
        }
    }
}

unsafe impl GlobalAlloc for BrkAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut ptr = self.allocator.alloc(layout);
        // FIXED: if alloc failed, ptr is null
        // FIXED: try to extend heap size, then alloc again
        if ptr.is_null() && self.extend(layout) {
            ptr = self.allocator.alloc(layout);
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.allocator.dealloc(ptr, layout)
    }
}

#[cfg(not(test))]
#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout)
}
