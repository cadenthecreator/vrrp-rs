use std::time::Instant;

use super::interval::Interval;

#[derive(Clone, Copy, Debug, PartialOrd, PartialEq)]
pub struct Priority(u8);

impl Priority {
    pub const SHUTDOWN: Priority = Priority(0);
    pub const OWNER: Priority = Priority(255);

    pub const fn as_u32(&self) -> u32 {
        self.0 as u32
    }

    pub const fn new(priority: u8) -> Self {
        Self(priority)
    }
}

impl Default for Priority {
    fn default() -> Self {
        Priority::new(100)
    }
}

impl Priority {
    pub fn skew_time(&self, master_adver_interval: Interval) -> Interval {
        ((256 - self.as_u32()) * master_adver_interval) / 256
    }

    pub fn master_down_interval(&self, master_adver_interval: Interval) -> Interval {
        3 * master_adver_interval + self.skew_time(master_adver_interval)
    }

    pub fn master_down_timer(&self, now: Instant, master_adver_interval: Interval) -> Instant {
        now + self.master_down_interval(master_adver_interval)
    }
}
