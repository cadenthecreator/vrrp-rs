use crate::Interval;
use pnet_base::MacAddr;
use std::net::Ipv4Addr;
use std::num::NonZeroU8;

#[derive(Debug, PartialEq)]
pub enum ReceivedPacket {
    ShutdownAdvertisement {
        max_advertise_interval: Interval,
    },
    Advertisement {
        sender_ip: Ipv4Addr,
        priority: NonZeroU8,
        max_advertise_interval: Interval,
    },
    RequestARP {
        sender_mac: MacAddr,
        sender_ip: Ipv4Addr,
        target_ip: Ipv4Addr,
    },
    IP {
        target_mac: MacAddr,
        target_ip: Ipv4Addr,
    },
}
