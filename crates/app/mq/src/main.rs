#![no_std]
#![no_main]

use lib::*;

extern crate lib;

const CAPACITY: usize = 4; // Buffer capacity
const PRODUCER_COUNT: usize = 8;
const CONSUMER_COUNT: usize = 8;
const MSG_PER_THREAD: usize = 10;

const MUTEX: u32 = 2001;
const EMPTY: u32 = 2002;
const FULL: u32 = 2003;

// buffer
static mut QUEUE: [usize; 16] = [0; 16];
static mut HEAD: usize = 0;
static mut TAIL: usize = 0;
static mut MSG_COUNT: usize = 0;

fn main() -> isize {
    sys_new_sem(MUTEX, 1);
    sys_new_sem(EMPTY, CAPACITY);
    sys_new_sem(FULL, 0);

    let mut pids = [0u16; PRODUCER_COUNT + CONSUMER_COUNT];

    println!("MQ capacity: {}", CAPACITY);

    // consumer
    for i in 0..CONSUMER_COUNT {
        let pid = sys_fork();
        if pid == 0 {
            do_consumer(i);
            sys_exit(0);
        } else {
            pids[i] = pid;
            println!("Process {} is created as consumer", pid);
        }
    }

    // producer
    for i in 0..PRODUCER_COUNT {
        let pid = sys_fork();
        if pid == 0 {
            do_producer(i);
            sys_exit(0);
        } else {
            pids[CONSUMER_COUNT + i] = pid;
            println!("Process {} is created as producer", pid);
        }
    }

    // // consumer
    // for i in 0..CONSUMER_COUNT {
    //     let pid = sys_fork();
    //     if pid == 0 {
    //         do_consumer(i);
    //         sys_exit(0);
    //     } else {
    //         pids[i] = pid;
    //         println!("Process {} is created as consumer", pid);
    //     }
    // }

    // print process status
    sys_stat();

    // wait for all producers and consumers to finish
    for pid in pids.iter() {
        sys_wait_pid(*pid);
    }

    println!("All finished. Remaining messages: {}", unsafe { MSG_COUNT });

    sys_remove_sem(MUTEX);
    sys_remove_sem(EMPTY);
    sys_remove_sem(FULL);
    0
}

// P for proberen, V for verhogen

fn do_producer(id: usize) {
    for msg in 0..MSG_PER_THREAD {
        let data = id * 1000 + msg; // construct a message

        sys_sem_wait(EMPTY); // P(empty)，wait for empty slot
        sys_sem_wait(MUTEX); // P(mutex)，enter critical section to modify queue

        unsafe {
            QUEUE[TAIL] = data;
            TAIL = (TAIL + 1) % CAPACITY;
            MSG_COUNT += 1;
            let count = unsafe { core::ptr::read_volatile(&raw const MSG_COUNT) };
            println!(
                "Producer {} pushed message: {} | Count: {}",
                id, data, count
            );
        }

        sys_sem_signal(MUTEX); // V(mutex)
        sys_sem_signal(FULL); // V(full)，a new message is available
    }
}

fn do_consumer(id: usize) {
    for _ in 0..MSG_PER_THREAD {
        sys_sem_wait(FULL); // P(full)，wait for messages in the queue
        sys_sem_wait(MUTEX); // P(mutex)

        unsafe {
            let data = QUEUE[HEAD];
            HEAD = (HEAD + 1) % CAPACITY;
            MSG_COUNT -= 1;
            let count = unsafe { core::ptr::read_volatile(&raw const MSG_COUNT) };
            println!(
                "Consumer {} popped message: {} | Count: {}",
                id, data, count
            );
        }
        sys_sem_signal(MUTEX); // V(mutex)
        sys_sem_signal(EMPTY); // V(empty)，available slot for producers
    }
}

entry!(main);
