use core::{
    hint::spin_loop,
    sync::atomic::{AtomicBool, Ordering},
};

use syscall_def::Syscall;

use crate::*;

pub struct SpinLock {
    bolt: AtomicBool,
}

impl SpinLock {
    pub const fn new() -> Self {
        Self {
            bolt: AtomicBool::new(false),
        }
    }

    pub fn acquire(&self) {
        // FIXED: acquire the lock, spin if the lock is not available
        while self
            .bolt
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            spin_loop();
        }
    }

    pub fn release(&self) {
        // FIXED: release the lock
        self.bolt.store(false, Ordering::Release);
    }
}

unsafe impl Sync for SpinLock {} // Why? Check reflection question 5

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Semaphore {
    key: u32,
}

impl Semaphore {
    pub const fn new(key: u32) -> Self {
        Semaphore { key }
    }

    #[inline(always)]
    pub fn init(&self, value: usize) -> bool {
        sys_new_sem(self.key, value)
    }

    /* FIXED: other functions with syscall... */
    #[inline(always)]
    pub fn remove(&self) -> bool {
        sys_remove_sem(self.key)
    }

    #[inline(always)]
    pub fn signal(&self) {
        sys_sem_signal(self.key);
    }

    #[inline(always)]
    pub fn wait(&self) {
        sys_sem_wait(self.key);
    }
}

unsafe impl Sync for Semaphore {}

#[macro_export]
macro_rules! semaphore_array {
    [$($x:expr),+ $(,)?] => {
        [ $($crate::sync::Semaphore::new($x),)* ]
    }
}
