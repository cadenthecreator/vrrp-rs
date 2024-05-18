use std::ops::{Add, Div, Mul};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval(u16);

impl Interval {
    pub const fn from_secs(seconds: u16) -> Self {
        Self::from_centis(10 * seconds)
    }

    pub const fn from_centis(centiseconds: u16) -> Self {
        Self(centiseconds)
    }
}

impl Into<Duration> for Interval {
    fn into(self) -> Duration {
        Duration::from_millis(self.0 as u64 * 10)
    }
}

impl Add<Interval> for Instant {
    type Output = Instant;

    fn add(self, rhs: Interval) -> Self::Output {
        self + <Interval as Into<Duration>>::into(rhs)
    }
}

impl Add<Interval> for Interval {
    type Output = Interval;

    fn add(self, rhs: Interval) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Mul<Interval> for u16 {
    type Output = Interval;

    fn mul(self, rhs: Interval) -> Self::Output {
        Interval(self * rhs.0)
    }
}

impl Div<u16> for Interval {
    type Output = Interval;

    fn div(self, rhs: u16) -> Self::Output {
        Interval(self.0 / rhs)
    }
}
