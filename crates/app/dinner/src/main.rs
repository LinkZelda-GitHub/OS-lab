#![no_std]
#![no_main]

use lib::*;

extern crate lib;

const PHILOSOPHER_COUNT: usize = 5;

const CHOPSTICKS: [u32; 5] = [3000, 3001, 3002, 3003, 3004];

const WAITER: u32 = 4000;

fn main() -> isize {
    sys_new_sem(WAITER, 4);
    for i in 0..PHILOSOPHER_COUNT {
        sys_new_sem(CHOPSTICKS[i], 1);
    }

    let mut pids = [0u16; PHILOSOPHER_COUNT];

    for i in 0..PHILOSOPHER_COUNT {
        let pid = sys_fork();
        if pid == 0 {
            philosopher(i);
            sys_exit(0);
        } else {
            pids[i] = pid;
        }
    }

    for pid in pids.iter() {
        sys_wait_pid(*pid);
    }

    println!("Dinner is over!");

    for i in 0..PHILOSOPHER_COUNT {
        sys_remove_sem(CHOPSTICKS[i]);
    }
    sys_remove_sem(WAITER);

    0
}

pub fn sleep(millisecs: usize) {
    let start = sys_time();
    let dur = millisecs;

    loop {
        let current = sys_time();
        if current - start >= dur {
            break;
        }
    }
}

fn philosopher(id: usize) {
    let left = id;
    let right = (id + 1) % PHILOSOPHER_COUNT;

    for _ in 0..3 {
        // Eating 3 times for each philosopher
        println!("Philosopher {} is thinking...", id);
        sleep(sys_get_pid() as usize * 100); // Thinking time

        println!("Philosopher {} is hungry...", id);

        // Get permission from the waiter to eat
        sys_sem_wait(WAITER);

        if id % 2 == 0 {
            sys_sem_wait(CHOPSTICKS[right]);
            println!("Philosopher {} picked up right chopstick {}", id, right);

            sleep(0x100);

            sys_sem_wait(CHOPSTICKS[left]);
            println!("Philosopher {} picked up left chopstick {}", id, left);
        } else {
            sys_sem_wait(CHOPSTICKS[left]);
            println!("Philosopher {} picked up left chopstick {}", id, left);

            sleep(0x10);

            sys_sem_wait(CHOPSTICKS[right]);
            println!("Philosopher {} picked up right chopstick {}", id, right);
        }

        // Eating
        println!("*** Philosopher {} is EATING! ***", id);
        sleep(0x40);

        // Finish
        sys_sem_signal(CHOPSTICKS[right]);
        sys_sem_signal(CHOPSTICKS[left]);

        // Leave the seat
        sys_sem_signal(WAITER);
    }
}

entry!(main);
