#![no_std]
#![no_main]

use lib::{sync::SpinLock, *};
extern crate lib;

// static SPIN_LOCK: SpinLock = SpinLock::new();

struct FishState {
    state: usize,
    dir: usize,
    done: usize,
    finished: bool,
}

static FISH_LOCK: SpinLock<FishState> = SpinLock::new(FishState {
    state: 0,
    dir: 0,
    done: 0,
    finished: false,
});

// STATE: 0 = init, 1: printed 1st char, 2 = printed 2nd char,
// 3 = printed 3rd char
// static mut STATE: usize = 0;
// direction: 0 = init, 1: '<' first, 2 = '>' first
// static mut DIR: usize = 0;
// done count
// static mut DONE: usize = 0;
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
        let mut ok = false;
        {
            let mut _guard = FISH_LOCK.lock();
            // SPIN_LOCK.acquire();
            if should_exit() {
                // SPIN_LOCK.release();
                break;
            }
            if _guard.state == 0 {
                print!("<");
                _guard.state = 1;
                _guard.dir = 1;
                ok = true;
            } else if _guard.state == 1 && _guard.dir == 2 {
                print!("<");
                _guard.state = 2;
                ok = true;
            } else if _guard.state == 2 && _guard.dir == 1 {
                print!("<");
                _guard.state = 3;
                ok = true;
            }
        }
        // SPIN_LOCK.release();
        if !ok {
            core::hint::spin_loop();
        }
    }
}

fn fish_greater() {
    loop {
        let mut ok = false;
        {
            let mut _guard = FISH_LOCK.lock();
            // SPIN_LOCK.acquire();
            if should_exit() {
                // SPIN_LOCK.release();
                break;
            }
            if _guard.state == 0 {
                print!(">");
                _guard.state = 1;
                _guard.dir = 2;
                ok = true;
            } else if _guard.state == 1 && _guard.dir == 1 {
                print!(">");
                _guard.state = 2;
                ok = true;
            } else if _guard.state == 2 && _guard.dir == 2 {
                print!(">");
                _guard.state = 3;
                ok = true;
            }
        }
        // SPIN_LOCK.release();
        if !ok {
            core::hint::spin_loop();
        }
    }
}

fn fish_underline() {
    loop {
        let mut ok = false;
        {
            let mut _guard = FISH_LOCK.lock();
            // FISH_LOCK.acquire();
            unsafe {
                if _guard.state == 3 {
                    println!("_");
                    _guard.state = 0;
                    _guard.dir = 0;
                    _guard.done += 1;
                    if _guard.done >= TARGET {
                        FINISHED = true;
                    }
                    ok = true;
                }
            }
        }
        // SPIN_LOCK.release();
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
