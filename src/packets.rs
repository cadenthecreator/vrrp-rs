use pnet_base::MacAddr;
use std::net::Ipv4Addr;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArpReply {
    pub sender_mac: MacAddr,
    pub sender_ip: Ipv4Addr,
    pub target_mac: MacAddr,
    pub target_ip: Ipv4Addr,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IpPacket {
    pub sender_ip: Ipv4Addr,
    pub target_ip: Ipv4Addr,
}
