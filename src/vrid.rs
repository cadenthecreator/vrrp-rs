use pnet_base::MacAddr;
use std::num::NonZeroU8;

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct VRID(NonZeroU8);

impl VRID {
    pub fn into_mac_address(self) -> MacAddr {
        // https://datatracker.ietf.org/doc/html/rfc9568#section-7.3
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
        MacAddr(0x00, 0x00, 0x5E, 0x00, 0x01, self.0.into())
    }
}

impl TryFrom<u8> for VRID {
    type Error = <NonZeroU8 as TryFrom<u8>>::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(VRID(value.try_into()?))
    }
}
