#![no_std]
#![allow(dead_code)]
#![feature(abi_x86_interrupt)]
#![feature(alloc_error_handler)]
#![feature(type_alias_impl_trait)]
#![feature(map_try_insert)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::result_unit_err)]

extern crate alloc;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate bitflags;
extern crate libm;

#[macro_use]
pub mod utils;
use core::mem;

pub use utils::*;

#[macro_use]
pub mod drivers;
pub use drivers::*;

pub mod interrupt;
pub mod memory;
pub mod proc;

pub use alloc::format;

use boot::BootInfo;
use storage::partition::{PartitionTable, mbr::MbrTable};
use uefi::{Status, runtime::ResetType};

pub fn init(boot_info: &'static BootInfo) {
    unsafe {
        // set uefi system table
        uefi::table::set_system_table(boot_info.system_table.cast().as_ptr());
    }

    serial::init(); // init serial output
    logger::init("Debug"); // init logger system
    memory::allocator::init(); // init kernel heap allocator
    memory::address::init(boot_info);
    memory::gdt::init(); // init gdt
    memory::init(boot_info); // init memory manager

    if let Some(ref gop_info) = boot_info.gop_info {
        // init vga driver
        vga::init(gop_info);
    }

    memory::user::init(); // init user heap allocator
    interrupt::init(); // init interrupts

    keyboard::init(); // init keyboard driver

    filesystem::init(); // init filesystem drivers
    proc::init(boot_info); // init process management

    x86_64::instructions::interrupts::enable();
    info!("Interrupts Enabled.");

    info!("YatSenOS initialized.");

    // crate::drivers::ata::AtaDrive::open(0, 0);
    info!(
        "Partition 0 Info:{:#?}",
        MbrTable::parse(crate::drivers::ata::AtaDrive::open(0, 0).unwrap())
            .unwrap()
            .partitions()
            .unwrap()
            .first()
            .unwrap()
    );

    info!("Test stack grow.");

    grow_stack();

    info!("Stack grow test done.");
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn grow_stack() {
    const STACK_SIZE: usize = 1024 * 4;
    const STEP: usize = 64;

    let mut array = [0u64; STACK_SIZE];
    info!("Stack: {:?}", array.as_ptr());

    // test write
    for i in (0..STACK_SIZE).step_by(STEP) {
        array[i] = i as u64;
    }

    // test read
    for i in (0..STACK_SIZE).step_by(STEP) {
        assert_eq!(array[i], i as u64);
    }
}

pub fn shutdown() -> ! {
    info!("YatSenOS shutting down.");
    uefi::runtime::reset(ResetType::SHUTDOWN, Status::SUCCESS, None);
}

pub fn wait(init: proc::ProcessId) {
    loop {
        if proc::still_alive(init) {
            // Why? Check reflection question 5
            x86_64::instructions::hlt();
        } else {
            break;
        }
    }
}
