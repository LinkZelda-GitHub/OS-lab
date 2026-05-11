use core::{
    hint::spin_loop,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

use syscall_def::Syscall;

use crate::*;

// pub struct SpinLock {
//     bolt: AtomicBool,
// }

// impl SpinLock {
//     pub const fn new() -> Self {
//         Self {
//             bolt: AtomicBool::new(false),
//         }
//     }

//     pub fn acquire(&self) {
//         // FIXED: acquire the lock, spin if the lock is not available
//         while self
//             .bolt
//             .compare_exchange(false, true, Ordering::Acquire,
// Ordering::Relaxed)             .is_err()
//         {
//             spin_loop();
//         }
//     }

//     pub fn release(&self) {
//         // FIXED: release the lock
//         self.bolt.store(false, Ordering::Release);
//     }
// }

pub struct SpinLock<T> {
    bolt: AtomicBool,
    data: core::cell::UnsafeCell<T>,
}

unsafe impl<T> Sync for SpinLock<T> {}
unsafe impl<T> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            bolt: AtomicBool::new(false),
            data: core::cell::UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> SpinLockGuard<'_, T> {
        // FIXED: acquire the lock, spin if the lock is not available
        while self
            .bolt
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            spin_loop();
        }
        SpinLockGuard { lock: self }
    }
}

pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

// impl Drop for SpinLockGuard {
//     fn drop(&mut self) {
//         self.lock.bolt.store(false, Ordering::Release);
//     }
// }

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.bolt.store(false, Ordering::Release);
    }
}

// unsafe impl Sync for SpinLock {} // Why? Check reflection question 5

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
