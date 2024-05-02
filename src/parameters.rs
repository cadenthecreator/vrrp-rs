use crate::{Interval, Priority};
use pnet_base::MacAddr;
use std::net::Ipv4Addr;
use std::num::NonZeroU8;

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
    pub(crate) fn mac_address(&self) -> MacAddr {
        // https://datatracker.ietf.org/doc/html/rfc5798#section-7.3
        //    The virtual router MAC address associated with a virtual router is an
        //    IEEE 802 MAC Address in the following format:
        //
        //    IPv4 case: 00-00-5E-00-01-{VRID} (in hex, in Internet-standard bit-
        //    order)
        //
        //    The first three octets are derived from the IANA's Organizational
        //    Unique Identifier (OUI).  The next two octets (00-01) indicate the
        //    address block assigned to the VRRP for IPv4 protocol. {VRID} is the
        //    VRRP Virtual Router Identifier.  This mapping provides for up to 255
        //    IPv4 VRRP routers on a network.
        //
        //    IPv6 case: 00-00-5E-00-02-{VRID} (in hex, in Internet-standard bit-
        //    order)
        //
        //    The first three octets are derived from the IANA's OUI.  The next two
        //    octets (00-02) indicate the address block assigned to the VRRP for
        //    IPv6 protocol. {VRID} is the VRRP Virtual Router Identifier.  This
        //    mapping provides for up to 255 IPv6 VRRP routers on a network.
        MacAddr(0x00, 0x00, 0x5E, 0x00, 0x01, self.vrid.into())
    }

    pub(crate) fn master_down_interval(&self, master_adver_interval: Interval) -> Interval {
        3 * master_adver_interval + self.skew_time(master_adver_interval)
    }
    pub(crate) fn skew_time(&self, master_adver_interval: Interval) -> Interval {
        ((256 - self.priority.as_u32()) * master_adver_interval) / 256
    }

    pub(crate) fn ipv4(&self, index: usize) -> Ipv4Addr {
        self.ipv4_addresses[index]
    }
}

pub type VRID = NonZeroU8;
