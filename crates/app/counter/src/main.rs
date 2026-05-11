#![no_std]
#![no_main]

use lib::*;

extern crate lib;

const THREAD_COUNT: usize = 8;
static mut COUNTER_SEMAPHORE: isize = 0;

fn main() -> isize {
    // test_spin();
    // test_semaphore();
    let pid = sys_fork();

    if pid == 0 {
        test_semaphore();
    } else {
        test_spin();
        sys_wait_pid(pid);
    }

    0
}

// static SPIN_LOCK: lib::sync::SpinLock<isize> = lib::sync::SpinLock::new();
static COUNTER_SPIN: lib::sync::SpinLock<isize> = lib::sync::SpinLock::new(0);

fn test_spin() {
    let mut pids = [0u16; THREAD_COUNT];

    println!("Start SpinLock test...");

    // 清零 COUNTER 用于测试
    *COUNTER_SPIN.lock() = 0;

    for i in 0..THREAD_COUNT {
        let pid = sys_fork();
        if pid == 0 {
            do_counter_inc_spin();
            sys_exit(0);
        } else {
            pids[i] = pid;
        }
    }

    for i in 0..THREAD_COUNT {
        sys_wait_pid(pids[i]);
    }

    println!("SpinLock COUNTER result: {}", *COUNTER_SPIN.lock());
}

fn do_counter_inc_spin() {
    for _ in 0..100 {
        // FIXED: protect the critical section
        // SPIN_LOCK.acquire();
        // let _guard = SPIN_LOCK.lock();
        inc_counter_spin();
        // SPIN_LOCK.release();
    }
}

fn test_semaphore() {
    let mut pids = [0u16; THREAD_COUNT];

    println!("Start Semaphore test...");

    sys_new_sem(2333, 1);
    // 清零 COUNTER 用于测试
    unsafe {
        COUNTER_SEMAPHORE = 0;
    }

    for i in 0..THREAD_COUNT {
        let pid = sys_fork();
        if pid == 0 {
            do_counter_inc_semaphore();
            sys_exit(0);
        } else {
            pids[i] = pid;
        }
    }

    for i in 0..THREAD_COUNT {
        sys_wait_pid(pids[i]);
    }

    println!("Semaphore COUNTER result: {}", unsafe { COUNTER_SEMAPHORE });
    sys_remove_sem(2333);
}

fn do_counter_inc_semaphore() {
    for _ in 0..100 {
        // FIXED: protect the critical section
        sys_sem_wait(2333);
        inc_counter_semaphore();
        sys_sem_signal(2333);
    }
}

/// Increment the counter
///
/// this function simulate a critical section by delay
/// DO NOT MODIFY THIS FUNCTION
fn inc_counter_spin() {
    let mut val_guard = COUNTER_SPIN.lock();
    delay();
    let mut val = *val_guard;
    delay();
    val += 1;
    delay();
    *val_guard = val;
}

fn inc_counter_semaphore() {
    unsafe {
        delay();
        let mut val = COUNTER_SEMAPHORE;
        delay();
        val += 1;
        delay();
        COUNTER_SEMAPHORE = val;
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
