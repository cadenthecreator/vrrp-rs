use crate::{Interval, Priority};
use pnet_base::MacAddr;
use std::net::Ipv4Addr;

#[derive(Debug, PartialEq)]
pub enum Input {
    Command(Command),
    Packet(ReceivedPacket),
    Timer,
}

#[derive(Debug, PartialEq)]
pub enum Command {
    Startup,
    Shutdown,
}

#[derive(Debug, PartialEq)]
pub enum ReceivedPacket {
    Advertisement(Priority, Interval),
    ARP {
        sender_mac: MacAddr,
        sender_ip: Ipv4Addr,
        target_ip: Ipv4Addr,
    },
    IpPacket {
       target_mac: MacAddr,
        target_ip: Ipv4Addr,
    },
}

impl From<Command> for Input {
    fn from(command: Command) -> Self {
        Self::Command(command)
    }
}

impl From<ReceivedPacket> for Input {
    fn from(oacket: ReceivedPacket) -> Self {
        Self::Packet(oacket)
    }
}
