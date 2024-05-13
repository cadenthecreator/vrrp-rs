use crate::{ArpReply, Interval, IpPacket, Parameters, Priority};
use pnet_base::MacAddr;
use std::net::Ipv4Addr;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action<'a> {
    Activate(&'a [Ipv4Addr]),
    Deactivate(&'a [Ipv4Addr]),
    SendAdvertisement(Priority, Interval),
    BroadcastGratuitousARP(MacAddr, Ipv4Addr),
    SendARP(ArpReply),
    AcceptPacket(IpPacket<'a>),
    ForwardPacket(IpPacket<'a>),
}

#[derive(Debug, PartialEq)]
pub enum Actions<'a> {
    TransitionToMaster(&'a Parameters, TransitionToMaster),
    ShutdownMaster(&'a Parameters, ShutdownMaster),
    OneAction(Option<Action<'a>>),
    None,
}

impl<'a> From<Action<'a>> for Actions<'a> {
    fn from(value: Action<'a>) -> Self {
        Actions::OneAction(Some(value))
    }
}

#[derive(Debug, PartialEq, Default)]
pub enum TransitionToMaster {
    #[default]
    Activate,
    Advertisment,
    NextARP(usize),
}
#[derive(Debug, PartialEq, Default)]
pub enum ShutdownMaster {
    #[default]
    Advertisment,
    Deactivate,
    Done,
}

impl<'a> Iterator for Actions<'a> {
    type Item = Action<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        use TransitionToMaster::*;
        match self {
            Actions::None => None,
            Actions::ShutdownMaster(parameters, shutdown) => match shutdown {
                ShutdownMaster::Advertisment => {
                    *shutdown = ShutdownMaster::Deactivate;
                    Some(Action::SendAdvertisement(
                        Priority::SHUTDOWN,
                        parameters.advertisement_interval,
                    ))
                }
                ShutdownMaster::Deactivate => {
                    *shutdown = ShutdownMaster::Done;
                    Some(Action::Deactivate(&parameters.ipv4_addresses))
                }
                ShutdownMaster::Done => None,
            },
            Actions::TransitionToMaster(parameters, transition) => match transition {
                Activate => {
                    *transition = Advertisment;
                    Some(Action::Activate(&parameters.ipv4_addresses))
                }
                Advertisment => {
                    *transition = NextARP(0);
                    Some(Action::SendAdvertisement(
                        parameters.priority,
                        parameters.advertisement_interval,
                    ))
                }
                NextARP(offset) if *offset < parameters.ipv4_addresses.len() => {
                    let next_address = parameters.ipv4_addresses[*offset];
                    *transition = NextARP(*offset + 1);
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
