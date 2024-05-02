use crate::{Interval, IpPacket, Priority};
use pnet_base::MacAddr;
use std::net::Ipv4Addr;
use std::time::Instant;

#[derive(Debug, PartialEq)]
pub enum Input<'a> {
    Advertisement(Instant, Priority, Interval),
    ARP {
        sender_mac: MacAddr,
        sender_ip: Ipv4Addr,
        target_ip: Ipv4Addr,
    },
    Startup(Instant),
    Timer(Instant),
    IpPacket(MacAddr, IpPacket<'a>),
    Shutdown,
}
