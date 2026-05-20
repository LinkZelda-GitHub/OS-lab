#![no_std]
#![no_main]

// use lib::*;
use ysos::*;
// extern crate lib;
use ysos_kernel::{self as ysos};

extern crate alloc;

boot::entry_point!(kernel_main);

pub fn kernel_main(boot_info: &'static boot::BootInfo) -> ! {
    ysos::init(boot_info);

    ysos::wait(spawn_init());
    // sys_exit(0);
    // unsafe {
    //     core::arch::asm!(
    //         "int 0x80",
    //         in("rax") 60,
    //         in("rdi") 0,
    //     );
    // }

    // loop {}
    ysos::shutdown();
}

pub fn spawn_init() -> proc::ProcessId {
    // NOTE: you may want to clear the screen before starting the shell
    // print!("\x1b[1;1H\x1b[2J");

    proc::list_app();
    proc::spawn("/APP/sh").unwrap()
}
