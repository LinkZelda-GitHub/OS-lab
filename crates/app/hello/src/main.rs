#![no_std]
#![no_main]

use lib::*;

extern crate lib;

fn main() -> isize {
    println!("Hello, world!!!");

    // let kernel_ptr = 0xffffff0000000000u64 as *const i32;
    // let secret = unsafe { *kernel_ptr };

    // println!("The secret number is: {}", secret);

    233
}

entry!(main);
