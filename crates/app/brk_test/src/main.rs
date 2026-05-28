// #![no_std]
// #![no_main]

// use lib::*;

// extern crate lib;

// const HEAP_SIZE: usize = 8 * 1024 - 8; // 8 KiB

// fn main() -> isize {
//     let heap_start = sys_brk(None).unwrap();
//     let heap_end = heap_start + HEAP_SIZE;

//     let ret = sys_brk(Some(heap_end)).expect("Failed to allocate heap");

//     assert!(ret == heap_end, "Failed to allocate heap");

//     0
// }

// entry!(main);

#![no_std]
#![no_main]
extern crate alloc;
use alloc::{boxed::Box, vec::Vec};

use lib::{entry, println};

fn main() -> isize {
    println!("Testing Automatic Heap Expansion...");

    let mut large_vec = Vec::new();
    for i in 0..10000 {
        large_vec.push(i);
    }

    let mut large_box = Box::new([0u8; 1024 * 1024]);

    large_box[0] = 42;
    large_box[1024 * 1024 - 1] = 84;

    // avoid optimization
    println!(
        "Large Box first element: {}, last element: {}",
        large_box[0],
        large_box[1024 * 1024 - 1]
    );

    println!(
        "Vec length: {}, Box alloc size: {}",
        large_vec.len(),
        large_box.len()
    );
    println!("Heap test PASSED!");

    0
}

entry!(main);
