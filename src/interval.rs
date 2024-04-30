use derive_more::{Add, Mul};
use std::ops::Div;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Add, Mul, PartialEq)]
pub struct Interval(u32);

impl Into<Duration> for Interval {
    fn into(self) -> Duration {
        Duration::from_millis(self.0 as u64 * 10)
    }
}

impl Interval {
    pub const fn from_secs(seconds: u32) -> Self {
        Self::from_centis(10 * seconds)
    }

    pub const fn from_centis(centiseconds: u32) -> Self {
        Self(centiseconds)
    }
}

impl Add<Interval> for Instant {
    type Output = Instant;

    fn add(self, rhs: Interval) -> Self::Output {
        self + <Interval as Into<Duration>>::into(rhs)
    }
}

impl Mul<Interval> for u32 {
    type Output = Interval;

    fn mul(self, rhs: Interval) -> Self::Output {
        Interval(self * rhs.0)
    }
}

impl Div<u32> for Interval {
    type Output = Interval;

    fn div(self, rhs: u32) -> Self::Output {
        Interval(self.0 / rhs)
    }
}
