use std::time::Instant;
use crate::Interval;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Priority(pub u8);

impl Priority {
    pub const SHUTDOWN: Priority = Priority(0);
    pub const OWNER: Priority = Priority(255);
}

impl Default for Priority {
    fn default() -> Self {
        Priority(100)
    }
}

impl Priority {
    pub fn skew_time(&self, master_adver_interval: Interval) -> Interval {
        ((256 - self.0 as u32) * master_adver_interval) / 256
    }

    pub fn master_down_interval(&self,master_adver_interval: Interval) -> Interval {
        3 * master_adver_interval + self.skew_time(master_adver_interval)
    }

    pub fn master_down_timer(&self, now: Instant, master_adver_interval: Interval) -> Instant {
        now + self.master_down_interval(master_adver_interval)
    }
}
