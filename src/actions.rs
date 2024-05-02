use crate::{ArpReply, Interval, IpPacket, Parameters, Priority};
use pnet_base::MacAddr;
use std::net::Ipv4Addr;

#[derive(Debug, PartialEq)]
pub enum Actions<'a> {
    TransitionToMaster {
        parameters: &'a Parameters,
        next_arp_offset: Option<usize>,
    },
    OneAction(Option<Action<'a>>),
    None,
}

impl<'a> From<Action<'a>> for Actions<'a> {
    fn from(value: Action<'a>) -> Self {
        Actions::OneAction(Some(value))
    }
}

impl<'a> Iterator for Actions<'a> {
    type Item = Action<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Actions::None => None,
            Actions::TransitionToMaster {
                parameters,
                next_arp_offset,
            } => match *next_arp_offset {
                None => {
                    *next_arp_offset = Some(0);
                    Some(Action::SendAdvertisement(
                        parameters.priority,
                        parameters.advertisement_interval,
                    ))
                }
                Some(offset) if offset < parameters.ipv4_addresses.len() => {
                    let next_address = parameters.ipv4_addresses[offset];
                    *next_arp_offset = Some(offset + 1);
                    Some(Action::BroadcastGratuitousARP(
                        parameters.mac_address(),
                        next_address,
                    ))
                }
                _ => None,
            },
            Actions::OneAction(action) => action.take(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action<'a> {
    SendAdvertisement(Priority, Interval),
    BroadcastGratuitousARP(MacAddr, Ipv4Addr),
    SendARP(ArpReply),
    AcceptPacket(IpPacket<'a>),
    ForwardPacket(IpPacket<'a>),
}
