#![no_std]
#![no_main]

use log::info;
use ysos::*;
use ysos_kernel::{self as ysos};

extern crate alloc;

boot::entry_point!(kernel_main);

pub fn kernel_main(boot_info: &'static boot::BootInfo) -> ! {
    ysos::init(boot_info);

    // Split-screen: top 1/3 = animated clock, bottom 2/3 = text console
    if boot_info.gop_info.is_some() {
        let clock_height = ysos::drivers::vga_text::init_vga_console_for_shell();

        // Spawn the clock process with clock area height info
        let mut clock_data = proc::ProcessData::new();
        clock_data.set_env("CLOCK_HEIGHT", &alloc::format!("{}", clock_height));
        let clock_pid = proc::spawn_kernel_thread(
            ysos::utils::func::clock_demo,
            alloc::string::String::from("clock"),
            Some(clock_data),
        );
        info!("Clock process spawned (pid={}, clock_height={})", clock_pid, clock_height);
    }

    ysos::wait(spawn_init());
    ysos::shutdown();
}

pub fn spawn_init() -> proc::ProcessId {
    proc::list_app();
    proc::spawn("/APP/sh").unwrap()
}
