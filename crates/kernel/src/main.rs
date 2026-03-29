// #![no_std]
// #![no_main]

// #[macro_use]
// extern crate log;

// use core::arch::asm;

// use ysos_kernel as ysos;

// boot::entry_point!(kernel_main);

// fn trigger_page_fault() {
//     unsafe {
//         let ptr = 0x11451499824 as *mut u8;
//         *ptr = 42; 
//     }
// }

// pub unsafe fn trigger_triple_fault() {
//     use x86_64::instructions::tables::lidt;
//     use x86_64::structures::DescriptorTablePointer;
//     use x86_64::VirtAddr;

//     // 1. 创建一个非法的 IDT 指针
//     let ptr = DescriptorTablePointer {
//         limit: 0,
//         base: VirtAddr::new(0),
//     };

//     // 2. 加载这个垃圾指针
//     lidt(&ptr);

//     // 3. 触发一个断点异常，瞬间引发三重错误
//     core::arch::asm!("int3");
// }

// pub unsafe fn trigger_divide_error() {
//     unsafe {
//         core::arch::asm!("mov eax, 0"); 
//         core::arch::asm!("div eax");
//     }
// }

// pub fn kernel_main(boot_info: &'static boot::BootInfo) -> ! {
//     ysos::init(boot_info);

//     // unsafe {
//     // trigger_triple_fault();
//     // }
//     // trigger_page_fault();
//     // unsafe {
//     //     trigger_divide_error();
//     // }

//     loop {
//         info!("Hello World from YatSenOS v2!");

//         for _ in 0..0x10000000 {
//             unsafe {
//                 asm!("nop");
//             }
//         }
//     }
// }


#![no_std]
#![no_main]

use ysos::*;
use ysos_kernel as ysos;

extern crate alloc;

boot::entry_point!(kernel_main);

pub fn kernel_main(boot_info: &'static boot::BootInfo) -> ! {
    ysos::init(boot_info);

    loop {
        print!("> ");
        let input = input::get_line();

        match input.trim() {
            "exit" => break,
            _ => {
                println!("You said: {}", input);
                println!("The counter value is {}", interrupt::clock::read_counter());
            }
        }
    }

    ysos::shutdown();
}