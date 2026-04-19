#![no_std]
#![no_main]

use lib::*;
extern crate lib;

pub fn sleep(millisecs: usize) {
    let start = sys_time();
    let dur = millisecs;

    loop {
        let current = sys_time();
        if (current - start >= dur) {
            break;
        }
    }
}

fn main() -> isize {
    let mut T = 5i32;
    while T > 0 {
        sleep(1000);
        println!("{} !", T);
        T -= 1;
    }
    println!("End!");
    233
}

entry!(main);
