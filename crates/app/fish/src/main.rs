#![no_std]
#![no_main]

use lib::{sync::SpinLock, *};
extern crate lib;

static SPIN_LOCK: SpinLock = SpinLock::new();

// 状态机记录
// STATE: 0=空闲, 1=已输出1个角标, 2=已输出2个角标, 3=已输出3个角标(等待下划线)
static mut STATE: usize = 0;
// DIR: 代表这一轮开头的方向。1 代表 '<' 开头， 2 代表 '>'开头
static mut DIR: usize = 0;

fn main() -> isize {
    let mut pids = [0u16; 3];
    for i in 0..3 {
        pids[i] = sys_fork();
        if pids[i] == 0 {
            match i {
                0 => fish_less(),
                1 => fish_greater(),
                2 => fish_underscore(),
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

fn fish_less() {
    for _ in 0..10 {
        // 打印 10 次
        loop {
            SPIN_LOCK.acquire();
            let mut matched = false;
            unsafe {
                if STATE == 0 {
                    print!("<");
                    STATE = 1;
                    DIR = 1;
                    matched = true;
                } else if STATE == 1 && DIR == 2 {
                    print!("<");
                    STATE = 2;
                    matched = true;
                } else if STATE == 2 && DIR == 1 {
                    print!("<");
                    STATE = 3;
                    matched = true;
                }
            }
            SPIN_LOCK.release();
            if matched {
                break;
            }
            core::hint::spin_loop();
        }
    }
}

fn fish_greater() {
    for _ in 0..10 {
        // 打印 10 次
        loop {
            SPIN_LOCK.acquire();
            let mut matched = false;
            unsafe {
                if STATE == 0 {
                    print!(">");
                    STATE = 1;
                    DIR = 2;
                    matched = true;
                } else if STATE == 1 && DIR == 1 {
                    print!(">");
                    STATE = 2;
                    matched = true;
                } else if STATE == 2 && DIR == 2 {
                    print!(">");
                    STATE = 3;
                    matched = true;
                }
            }
            SPIN_LOCK.release();
            if matched {
                break;
            }
            core::hint::spin_loop();
        }
    }
}

fn fish_underscore() {
    for _ in 0..10 {
        loop {
            SPIN_LOCK.acquire();
            let mut matched = false;
            unsafe {
                // 只有等前三个字符打印满了，才允许放行打印 '_'
                if STATE == 3 {
                    println!("_");
                    STATE = 0;
                    DIR = 0;
                    matched = true;
                }
            }
            SPIN_LOCK.release();
            if matched {
                break;
            }
            core::hint::spin_loop();
        }
    }
}

entry!(main);
