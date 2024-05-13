use crate::{Interval, IpPacket, Priority};
use pnet_base::MacAddr;
use std::net::Ipv4Addr;

#[derive(Debug, PartialEq)]
pub enum Input {
    Command(Command),
    Packet(Packet),
    Timer,
}

#[derive(Debug, PartialEq)]
pub enum Command {
    Startup,
    Shutdown,
}

#[derive(Debug, PartialEq)]
pub enum Packet {
    Advertisement(Priority, Interval),
    ARP {
        sender_mac: MacAddr,
        sender_ip: Ipv4Addr,
        target_ip: Ipv4Addr,
    },
    IpPacket(MacAddr, IpPacket),
}

impl From<Command> for Input {
    fn from(command: Command) -> Self {
        Self::Command(command)
    }
}

impl From<Packet> for Input {
    fn from(oacket: Packet) -> Self {
        Self::Packet(oacket)
    }
}
