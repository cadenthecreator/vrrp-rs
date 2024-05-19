use crate::{Interval, Mode, VRID};
use pnet_base::MacAddr;
use std::net::Ipv4Addr;

#[derive(Clone, Debug, PartialEq)]
pub struct Parameters {
    pub vrid: VRID,
    pub mode: Mode,
    pub virtual_addresses: Vec<Ipv4Addr>,
    pub advertisement_interval: Interval,
}

impl Parameters {
    pub fn new(vrid: VRID, virtual_addresses: Vec<Ipv4Addr>, mode: Mode) -> Self {
        Self {
            vrid,
            mode,
            virtual_addresses,
            advertisement_interval: Interval::from_secs(100),
        }
    }

    pub fn with_mode(self, mode: Mode) -> Self {
        Self { mode, ..self }
    }

    pub(crate) fn priority(&self) -> u16 {
        self.mode.priority().get() as u16
    }

    pub(crate) fn active_down_interval(&self, active_adver_interval: Interval) -> Interval {
        3 * active_adver_interval + self.skew_time(active_adver_interval)
    }

    pub(crate) fn skew_time(&self, active_adver_interval: Interval) -> Interval {
        ((256 - self.priority()) * active_adver_interval) / 256
    }

    pub(crate) fn mac_address(&self) -> MacAddr {
        self.vrid.into_mac_address()
    }

    #[cfg(test)]
    pub(crate) fn ipv4(&self, index: usize) -> Ipv4Addr {
        self.virtual_addresses[index]
    }
}
