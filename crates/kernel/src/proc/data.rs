use alloc::sync::Arc;

use hashbrown::HashMap;
use spin::RwLock;

use super::*;
use crate::{proc::sync::SemaphoreSet, resource::ResourceSet};

#[derive(Debug, Clone)]
pub struct ProcessData {
    // shared data
    pub(super) env: Arc<RwLock<HashMap<String, String, ahash::RandomState>>>,
    pub(super) resources: Arc<RwLock<ResourceSet>>,
    pub semaphores: Arc<RwLock<SemaphoreSet>>,
}

impl Default for ProcessData {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessData {
    pub fn new() -> Self {
        Self {
            env: Arc::new(RwLock::new(HashMap::default())),
            resources: Arc::new(RwLock::new(ResourceSet::default())),
            semaphores: Arc::new(RwLock::new(SemaphoreSet::default())),
        }
    }

    pub fn env(&self, key: &str) -> Option<String> {
        self.env.read().get(key).cloned()
    }

    pub fn set_env(&mut self, key: &str, val: &str) {
        self.env.write().insert(key.into(), val.into());
    }

    pub fn read(&self, fd: u8, buf: &mut [u8]) -> isize {
        self.resources.read().read(fd, buf)
    }

    pub fn write(&self, fd: u8, buf: &[u8]) -> isize {
        self.resources.write().write(fd, buf)
    }
}
