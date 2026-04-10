// use core::ptr::addr_of_mut;

// use linked_list_allocator::LockedHeap;
// use x86_64::VirtAddr;

// pub const HEAP_SIZE: usize = 8 * 1024 * 1024; // 8 MiB

// /// Use linked_list_allocator for kernel heap
// #[global_allocator]
// pub static ALLOCATOR: LockedHeap = LockedHeap::empty();

// pub fn init() {
//     // static buffer for kernel heap
//     // will be allocated on the bss section when the kernel is load
//     static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

//     let heap_start = VirtAddr::from_ptr(addr_of_mut!(HEAP));
//     let heap_end = heap_start + HEAP_SIZE as u64;

//     unsafe {
//         ALLOCATOR
//             .lock()
//             .init(addr_of_mut!(HEAP) as *mut u8, HEAP_SIZE);
//     }

//     debug!(
//         "Kernel Heap      : 0x{:016x}-0x{:016x}",
//         heap_start.as_u64(),
//         heap_end.as_u64()
//     );

//     let (size, unit) = crate::humanized_size(HEAP_SIZE as u64);
//     info!("Kernel Heap Size : {:>7.*} {}", 3, size, unit);

//     info!("Kernel Heap Initialized.");
// }

use core::ptr::addr_of_mut;
use core::alloc::{GlobalAlloc, Layout};
use linked_list_allocator::LockedHeap;
use x86_64::VirtAddr;
use log::debug;

pub const HEAP_SIZE: usize = 8 * 1024 * 1024; // 8 MiB

/// 实现一个支持关中断的分配器外壳，防止时钟中断切断分配过程造成死锁
pub struct SafeAllocator(pub LockedHeap);

unsafe impl GlobalAlloc for SafeAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // 在拿取自旋锁分配内存的整个过程中，强行关闭时钟中断！
        x86_64::instructions::interrupts::without_interrupts(|| {
            self.0.lock().allocate_first_fit(layout).ok().map_or(core::ptr::null_mut(), |allocation| allocation.as_ptr())
        })
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // 释放内存时同样屏蔽中断
        x86_64::instructions::interrupts::without_interrupts(|| {
            self.0.lock().deallocate(core::ptr::NonNull::new(ptr).unwrap(), layout)
        })
    }
}

/// 替换系统默认的 ALLOCATOR
#[global_allocator]
pub static ALLOCATOR: SafeAllocator = SafeAllocator(LockedHeap::empty());

pub fn init() {
    // static buffer for kernel heap
    static mut HEAP: [u8; HEAP_SIZE] = [0; HEAP_SIZE];

    let heap_start = VirtAddr::from_ptr(addr_of_mut!(HEAP));
    let _heap_end = heap_start + HEAP_SIZE as u64;

    unsafe {
        // 初始化时对内部的 LockedHeap (即 .0) 执行 init
        ALLOCATOR
            .0
            .lock()
            .init(addr_of_mut!(HEAP) as *mut u8, HEAP_SIZE);
    }

    debug!("Kernel Heap Initialized.");
}

#[alloc_error_handler]
fn alloc_error_handler(layout: alloc::alloc::Layout) -> ! {
    panic!("Allocation error: {:?}", layout);
}

/*
为什么会卡在这一行？
在支持多任务和时钟中断的内核里，如果使用基础的自旋锁（spin::Mutex）去保护全局堆（LockedHeap），会发生以下致命的死锁链：

你的主线程或是某个任务刚进入 alloc() 函数，成功拿到了堆空间的自旋锁，正准备划拨内存。
此时，时钟中断触发！主任务被强行挂起，调度器把 CPU 切给了后台闲逛的进程（或是另一个刚唤醒的终端线程）。
这个新任务也想要一点内存（比如申请一个局部变量、字符串等）。
新任务去拿堆分配器的锁。但锁还在那个被挂起的主线程手里！由于是单核环境，新线程一旦自旋就永远出不去，系统当场死机，再也无法输入。
怎么彻底解决？
必须重写全局分配器，确保在每次 alloc 和 dealloc 时，强制关闭系统的硬件时钟中断，让当前线程“不受任何打扰地”切完内存再放开锁！
*/