// #![no_std]
// #![no_main]

// #[macro_use]
// extern crate log;

// use core::arch::asm;

// use ysos_kernel as ysos;

// boot::entry_point!(kernel_main);

// pub fn kernel_main(boot_info: &'static boot::BootInfo) -> ! {
//     ysos::init(boot_info);

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