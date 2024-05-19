use crate::{BackupMode, Interval, Mode, VirtualAddresses, VRID};
use pnet_base::MacAddr;
use std::net::Ipv4Addr;

#[derive(Clone, Debug, PartialEq)]
pub struct Parameters {
    pub vrid: VRID,
    pub mode: Mode,
    pub virtual_addresses: VirtualAddresses,
    pub advertisement_interval: Interval,
}

impl Parameters {
    pub fn new(
        vrid: VRID,
        virtual_addresses: impl Into<VirtualAddresses>,
        mode: impl Into<Mode>,
    ) -> Self {
        Self {
            vrid,
            mode: mode.into(),
            virtual_addresses: virtual_addresses.into(),
            advertisement_interval: Interval::from_secs(100),
        }
    }

    pub fn with_mode(self, mode: Mode) -> Self {
        Self { mode, ..self }
    }

    pub(crate) fn primary_ip(&self) -> Ipv4Addr {
        match self.mode {
            Mode::Owner => self.virtual_addresses.get(0).unwrap(),
            Mode::Backup(BackupMode { primary_ip, .. }) => primary_ip,
        }
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
}
