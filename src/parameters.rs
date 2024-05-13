use crate::{Interval, Priority, VRID};
use pnet_base::MacAddr;
use std::net::Ipv4Addr;

#[derive(Clone, Debug, PartialEq)]
pub struct Parameters {
    pub vrid: VRID,
    pub priority: Priority,
    pub ipv4_addresses: Vec<Ipv4Addr>,
    pub advertisement_interval: Interval,
    pub preempt_mode: bool,
    pub accept_mode: bool,
}

impl Parameters {
    pub(crate) fn active_down_interval(&self, active_adver_interval: Interval) -> Interval {
        3 * active_adver_interval + self.skew_time(active_adver_interval)
    }

    pub(crate) fn skew_time(&self, active_adver_interval: Interval) -> Interval {
        ((256 - self.priority.as_u32()) * active_adver_interval) / 256
    }

    pub(crate) fn mac_address(&self) -> MacAddr {
        self.vrid.into_mac_address()
    }

    pub(crate) fn ipv4(&self, index: usize) -> Ipv4Addr {
        self.ipv4_addresses[index]
    }
}
