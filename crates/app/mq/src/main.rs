#![no_std]
#![no_main]

use lib::{
    sync::{Semaphore, SpinLock},
    *,
};

extern crate lib;

const CAPACITY: usize = 4; // 你可以通过修改这个值来测试不同容量 (1, 4, 8, 16)
const PRODUCER_COUNT: usize = 8;
const CONSUMER_COUNT: usize = 8;
const MSG_PER_THREAD: usize = 10;

static MUTEX: Semaphore = Semaphore::new(2001);
static EMPTY: Semaphore = Semaphore::new(2002);
static FULL: Semaphore = Semaphore::new(2003);

// 循环队列作为消息缓冲区
static mut QUEUE: [usize; 16] = [0; 16];
static mut HEAD: usize = 0;
static mut TAIL: usize = 0;
static mut MSG_COUNT: usize = 0;

fn main() -> isize {
    // 互斥锁始为 1
    MUTEX.init(1);
    // 最初空闲槽位为 CAPACITY
    EMPTY.init(CAPACITY);
    // 最初已有消息数为 0
    FULL.init(0);

    let mut pids = [0u16; PRODUCER_COUNT + CONSUMER_COUNT];

    println!("MQ capacity: {}", CAPACITY);

    // 创建消费者
    for i in 0..CONSUMER_COUNT {
        let pid = sys_fork();
        if pid == 0 {
            do_consumer(i);
            sys_exit(0);
        } else {
            pids[i] = pid;
        }
    }

    // 创建生产者
    for i in 0..PRODUCER_COUNT {
        let pid = sys_fork();
        if pid == 0 {
            do_producer(i);
            sys_exit(0);
        } else {
            pids[CONSUMER_COUNT + i] = pid;
        }
    }

    // 父进程输出全部状态记录
    sys_stat();

    // 等待所有子进程退出
    for pid in pids.iter() {
        sys_wait_pid(*pid);
    }

    println!("All finished. Remaining messages: {}", unsafe { MSG_COUNT });

    MUTEX.remove();
    EMPTY.remove();
    FULL.remove();
    0
}

fn do_producer(id: usize) {
    for msg in 0..MSG_PER_THREAD {
        let data = id * 1000 + msg; // 构建消息内容

        EMPTY.wait(); // P(empty)，等待空闲容量
        MUTEX.wait(); // P(mutex)，互斥修改队列

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

        MUTEX.signal(); // V(mutex)
        FULL.signal(); // V(full)，通知消费者有新消息了
    }
}

fn do_consumer(id: usize) {
    for _ in 0..MSG_PER_THREAD {
        FULL.wait(); // P(full)，等待队列有消息
        MUTEX.wait(); // P(mutex)

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

        MUTEX.signal(); // V(mutex)
        EMPTY.signal(); // V(empty)，通知生产者有新空余位了
    }
}

entry!(main);
