// use alloc::boxed::Box;

// use boot::{BootMemoryMap, MemoryType};
// use x86_64::{
//     PhysAddr,
//     structures::paging::{FrameAllocator, FrameDeallocator, PhysFrame,
// Size4KiB}, };

// once_mutex!(pub FRAME_ALLOCATOR: BootInfoFrameAllocator);

// guard_access_fn! {
//     pub get_frame_alloc(FRAME_ALLOCATOR: BootInfoFrameAllocator)
// }

// type BootInfoFrameIter = Box<dyn Iterator<Item = PhysFrame> + Send>;

// /// A FrameAllocator that returns usable frames from the bootloader's memory
// /// map.
// pub struct BootInfoFrameAllocator {
//     size: usize,
//     used: usize,
//     frames: BootInfoFrameIter,
// }

// impl BootInfoFrameAllocator {
//     /// Create a FrameAllocator from the passed memory map.
//     ///
//     /// This function is unsafe because the caller must guarantee that the
//     /// passed memory map is valid. The main requirement is that all frames
//     /// that are marked as `USABLE` in it are really unused.
//     pub unsafe fn init(memory_map: &BootMemoryMap, size: usize) -> Self {
//         BootInfoFrameAllocator {
//             size,
//             frames: create_frame_iter(memory_map),
//             used: 0,
//         }
//     }

//     pub fn frames_used(&self) -> usize {
//         self.used
//     }

//     pub fn frames_total(&self) -> usize {
//         self.size
//     }
// }

// unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
//     fn allocate_frame(&mut self) -> Option<PhysFrame> {
//         self.used += 1;
//         self.frames.next()
//     }
// }

// impl FrameDeallocator<Size4KiB> for BootInfoFrameAllocator {
//     unsafe fn deallocate_frame(&mut self, _frame: PhysFrame) {
//         // TODO: deallocate frame (not for lab 2)
//     }
// }

// fn create_frame_iter(memory_map: &BootMemoryMap) -> BootInfoFrameIter {
//     let iter = memory_map
//         .clone()
//         .into_iter()
//         // get usable regions from memory map
//         .filter(|r| r.ty == MemoryType::CONVENTIONAL)
//         // align to page boundary
//         .flat_map(|r| (0..r.page_count).map(move |v| (v * 4096 +
// r.phys_start)))         // create `PhysFrame` types from the start addresses
//         .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)));

//     Box::new(iter)
// }

use boot::{BootMemoryMap, MemoryType};
use x86_64::{
    PhysAddr,
    structures::paging::{FrameAllocator, FrameDeallocator, PhysFrame, Size4KiB},
};

once_mutex!(pub FRAME_ALLOCATOR: BootInfoFrameAllocator);

guard_access_fn! {
    pub get_frame_alloc(FRAME_ALLOCATOR: BootInfoFrameAllocator)
}

#[derive(Clone, Copy)]
struct CachedRegion {
    phys_start: u64,
    page_count: u64,
}

pub struct BootInfoFrameAllocator {
    size: usize,
    used: usize,
    regions: [CachedRegion; 64],
    regions_count: usize,
    current_region: usize,
    current_page_idx: u64,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &BootMemoryMap, size: usize) -> Self {
        let mut allocator = BootInfoFrameAllocator {
            size,
            used: 0,
            regions: [CachedRegion {
                phys_start: 0,
                page_count: 0,
            }; 64],
            regions_count: 0,
            current_region: 0,
            current_page_idx: 0,
        };

        // 仅将合格的空闲区间整理为一个最多容纳 64 个间隙的小数组缓存
        // 从而完美避开体积过大和迭代器闭包失效的双重陷阱！
        for region in memory_map.clone().into_iter() {
            if region.ty == MemoryType::CONVENTIONAL {
                if allocator.regions_count < 64 {
                    allocator.regions[allocator.regions_count] = CachedRegion {
                        phys_start: region.phys_start,
                        page_count: region.page_count,
                    };
                    allocator.regions_count += 1;
                }
            }
        }
        allocator
    }

    pub fn frames_used(&self) -> usize {
        self.used
    }

    pub fn frames_total(&self) -> usize {
        self.size
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        while self.current_region < self.regions_count {
            let region = &self.regions[self.current_region];

            if self.current_page_idx < region.page_count {
                let addr = region.phys_start + self.current_page_idx * 4096;
                self.current_page_idx += 1;
                self.used += 1;
                return Some(PhysFrame::containing_address(PhysAddr::new(addr)));
            }

            // 当前区域用完，跳往下一个物理内存可用碎片
            self.current_region += 1;
            self.current_page_idx = 0;
        }
        None
    }
}

impl FrameDeallocator<Size4KiB> for BootInfoFrameAllocator {
    unsafe fn deallocate_frame(&mut self, _frame: PhysFrame) {
        // TODO: deallocate frame (not for lab 2)
    }
}
