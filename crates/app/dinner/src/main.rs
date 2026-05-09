#![no_std]
#![no_main]

use lib::*;

extern crate lib;

use lib::sync::{Semaphore, SpinLock};

const PHILOSOPHER_COUNT: usize = 5;

// 定义5把筷子互斥锁
static CHOPSTICKS: [Semaphore; 5] = semaphore_array![3000, 3001, 3002, 3003, 3004];

// 服务生：只允许 4 个人同时进入“等待拿筷子”的竞争区，
// 防止全部5个人同时拿左筷子死锁。
static WAITER: Semaphore = Semaphore::new(4000);

fn main() -> isize {
    for i in 0..PHILOSOPHER_COUNT {
        CHOPSTICKS[i].init(1);
    }
    WAITER.init(4); // 服务生最多允许4人

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
        CHOPSTICKS[i].remove();
    }
    WAITER.remove();

    0
}

fn philosopher(id: usize) {
    let left = id;
    let right = (id + 1) % PHILOSOPHER_COUNT;

    for _ in 0..3 {
        // 每位哲学家思考和吃饭3次
        println!("Philosopher {} is thinking...", id);
        delay(sys_get_pid() as usize * 100); // 用 pid 来做伪随机休眠干扰

        println!("Philosopher {} is hungry...", id);

        // 向服务生请求上座
        WAITER.wait();

        CHOPSTICKS[left].wait();
        println!("Philosopher {} picked up left chopstick {}", id, left);

        delay(50); // 增加拿筷子间隙，容易诱发潜在竞态问题

        CHOPSTICKS[right].wait();
        println!("Philosopher {} picked up right chopstick {}", id, right);

        // 成功吃到饭
        println!("*** Philosopher {} is EATING! ***", id);
        delay(300);

        // 放下筷子
        CHOPSTICKS[right].signal();
        CHOPSTICKS[left].signal();

        // 离席，告诉服务生资源腾出了
        WAITER.signal();
    }
}

#[inline(never)]
#[unsafe(no_mangle)]
fn delay(cycles: usize) {
    for _ in 0..(cycles * 100) {
        core::hint::spin_loop();
    }
}

entry!(main);
