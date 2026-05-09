// #![no_std]
// #![no_main]

// use lib::*;

// extern crate lib;

// const THREAD_COUNT: usize = 8;
// static mut COUNTER: isize = 0;

// fn main() -> isize {
//     let mut pids = [0u16; THREAD_COUNT];

//     for i in 0..THREAD_COUNT {
//         let pid = sys_fork();
//         if pid == 0 {
//             do_counter_inc();
//             sys_exit(0);
//         } else {
//             pids[i] = pid; // only parent knows child's pid
//         }
//     }

//     let cpid = sys_get_pid();
//     println!("process #{} holds threads: {:?}", cpid, &pids);
//     sys_stat();

//     for i in 0..THREAD_COUNT {
//         println!("#{} waiting for #{}...", cpid, pids[i]);
//         sys_wait_pid(pids[i]);
//     }

//     println!("COUNTER result: {}", unsafe { COUNTER });

//     0
// }

// fn do_counter_inc() {
//     for _ in 0..100 {
//         // FIXME: protect the critical section
//         inc_counter();
//     }
// }

#![no_std]
#![no_main]

use lib::*;

extern crate lib;

use lib::sync::{Semaphore, SpinLock};

const THREAD_COUNT: usize = 8;
static mut COUNTER: isize = 0;

static SPIN_LOCK: SpinLock = SpinLock::new();
static SEMAPHORE: Semaphore = Semaphore::new(1001); // 随意选一个全局键值

fn main() -> isize {
    // 初始化信号量互斥锁的值为 1
    SEMAPHORE.init(1);

    test_spin();
    test_semaphore();
    // let pid = sys_fork();

    // if pid == 0 {
    //     test_semaphore();
    //     sys_exit(0);
    // } else {
    //     test_spin();
    //     sys_wait_pid(pid);
    // }

    0
}

fn test_spin() {
    let mut pids = [0u16; THREAD_COUNT];

    println!("Start SpinLock test...");

    // 清零 COUNTER 用于测试
    unsafe {
        COUNTER = 0;
    }

    for i in 0..THREAD_COUNT {
        let pid = sys_fork();
        if pid == 0 {
            for _ in 0..100 {
                // 临界区外
                SPIN_LOCK.acquire();
                // 临界区内
                inc_counter();
                SPIN_LOCK.release();
            }
            sys_exit(0);
        } else {
            pids[i] = pid;
        }
    }

    for i in 0..THREAD_COUNT {
        sys_wait_pid(pids[i]);
    }

    println!("SpinLock COUNTER result: {}", unsafe { COUNTER });
}

fn test_semaphore() {
    let mut pids = [0u16; THREAD_COUNT];

    println!("Start Semaphore test...");

    // 清零 COUNTER 用于测试
    unsafe {
        COUNTER = 0;
    }

    for i in 0..THREAD_COUNT {
        let pid = sys_fork();
        if pid == 0 {
            for _ in 0..100 {
                // P操作（Wait）获取配额
                SEMAPHORE.wait();
                inc_counter();
                // V操作（Signal）释放配额
                SEMAPHORE.signal();
            }
            sys_exit(0);
        } else {
            pids[i] = pid;
        }
    }

    for i in 0..THREAD_COUNT {
        sys_wait_pid(pids[i]);
    }

    println!("Semaphore COUNTER result: {}", unsafe { COUNTER });
}

/// Increment the counter
///
/// this function simulate a critical section by delay
/// DO NOT MODIFY THIS FUNCTION
fn inc_counter() {
    unsafe {
        delay();
        let mut val = COUNTER;
        delay();
        val += 1;
        delay();
        COUNTER = val;
    }
}

#[inline(never)]
#[unsafe(no_mangle)]
fn delay() {
    for _ in 0..0x100 {
        core::hint::spin_loop();
    }
}

entry!(main);
