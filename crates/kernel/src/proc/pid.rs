use core::sync::atomic::{AtomicU16, Ordering};

static PID_COUNTER: AtomicU16 = AtomicU16::new(1);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessId(pub u16);

impl ProcessId {
    pub fn new() -> Self {
        // FIXED: Get a unique PID
        let pid = PID_COUNTER.fetch_add(1, Ordering::Relaxed);
        Self(pid)
    }
}

impl Default for ProcessId {
    fn default() -> Self {
        Self::new()
    }
}

impl core::fmt::Display for ProcessId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::fmt::Debug for ProcessId {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ProcessId> for u16 {
    fn from(pid: ProcessId) -> Self {
        pid.0
    }
}
