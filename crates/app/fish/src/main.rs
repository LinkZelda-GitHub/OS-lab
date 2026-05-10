#![no_std]
#![no_main]

use lib::{sync::SpinLock, *};
extern crate lib;

static SPIN_LOCK: SpinLock = SpinLock::new();

// STATE: 0 = init, 1: printed 1st char, 2 = printed 2nd char,
// 3 = printed 3rd char
static mut STATE: usize = 0;
// direction: 0 = init, 1: '<' first, 2 = '>' first
static mut DIR: usize = 0;
// done count
static mut DONE: usize = 0;
// ended flag
static mut FINISHED: bool = false;

const TARGET: usize = 10; // number of fish

fn main() -> isize {
    let mut pids = [0u16; 3];
    for i in 0..3 {
        pids[i] = sys_fork();
        if pids[i] == 0 {
            match i {
                0 => fish_less(),
                1 => fish_greater(),
                2 => fish_underline(),
                _ => {}
            }
            sys_exit(0);
        }
    }

    for i in 0..3 {
        sys_wait_pid(pids[i]);
    }
    0
}

fn should_exit() -> bool {
    unsafe { FINISHED }
}

fn fish_less() {
    loop {
        SPIN_LOCK.acquire();
        if should_exit() {
            SPIN_LOCK.release();
            break;
        }
        let mut ok = false;
        unsafe {
            if STATE == 0 {
                print!("<");
                STATE = 1;
                DIR = 1;
                ok = true;
            } else if STATE == 1 && DIR == 2 {
                print!("<");
                STATE = 2;
                ok = true;
            } else if STATE == 2 && DIR == 1 {
                print!("<");
                STATE = 3;
                ok = true;
            }
        }
        SPIN_LOCK.release();
        if !ok {
            core::hint::spin_loop();
        }
    }
}

fn fish_greater() {
    loop {
        SPIN_LOCK.acquire();
        if should_exit() {
            SPIN_LOCK.release();
            break;
        }
        let mut ok = false;
        unsafe {
            if STATE == 0 {
                print!(">");
                STATE = 1;
                DIR = 2;
                ok = true;
            } else if STATE == 1 && DIR == 1 {
                print!(">");
                STATE = 2;
                ok = true;
            } else if STATE == 2 && DIR == 2 {
                print!(">");
                STATE = 3;
                ok = true;
            }
        }
        SPIN_LOCK.release();
        if !ok {
            core::hint::spin_loop();
        }
    }
}

fn fish_underline() {
    loop {
        SPIN_LOCK.acquire();
        let mut ok = false;
        unsafe {
            if STATE == 3 {
                println!("_");
                STATE = 0;
                DIR = 0;
                DONE += 1;
                if DONE >= TARGET {
                    FINISHED = true;
                }
                ok = true;
            }
        }
        SPIN_LOCK.release();
        if ok {
            if should_exit() {
                break;
            }
        } else {
            core::hint::spin_loop();
        }
    }
}

entry!(main);
