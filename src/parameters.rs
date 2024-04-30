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
    pub fn ipv4(&self, index: usize) -> Ipv4Addr {
        self.ip_addresses[index]
    }
}
