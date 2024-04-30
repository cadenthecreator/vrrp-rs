use crate::{Interval, Priority};
use pnet_base::MacAddr;
use std::net::Ipv4Addr;

#[derive(Clone, Debug, PartialEq)]
pub struct RouterParameters {
    pub mac_address: MacAddr,
    pub ip_addresses: Vec<Ipv4Addr>,
    pub priority: Priority,
    pub advertisement_interval: Interval,
}

impl RouterParameters {
    pub(crate) fn ipv4(&self, index: usize) -> Ipv4Addr {
        self.ip_addresses[index]
    }

    pub(crate) fn master_down_interval(&self, master_adver_interval: Interval) -> Interval {
        3 * master_adver_interval + self.skew_time(master_adver_interval)
    }

    pub(crate) fn skew_time(&self, master_adver_interval: Interval) -> Interval {
        ((256 - self.priority.as_u32()) * master_adver_interval) / 256
    }
}
