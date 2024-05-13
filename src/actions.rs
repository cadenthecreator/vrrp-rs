use crate::{ArpReply, Interval, Parameters, Priority};
use pnet_base::MacAddr;
use std::net::Ipv4Addr;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Activate,
    Deactivate,
    Send(SendPacket),
    Route(RoutePacket),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RoutePacket {
    Reject,
    Accept,
    Forward,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SendPacket {
    Advertisement(Priority, Interval),
    GratuitousARP(MacAddr, Ipv4Addr),
    ARP(ArpReply),
}

impl From<RoutePacket> for Action {
    fn from(value: RoutePacket) -> Self {
        Self::Route(value)
    }
}

impl From<SendPacket> for Action {
    fn from(value: SendPacket) -> Self {
        Self::Send(value)
    }
}

#[derive(Debug, PartialEq)]
pub enum Actions<'a> {
    TransitionToActive(&'a Parameters, TransitionToActive),
    ShutdownActive(&'a Parameters, ShutdownActive),
    OneAction(Option<Action>),
    None,
}

impl<'a> From<Action> for Actions<'a> {
    fn from(value: Action) -> Self {
        Actions::OneAction(Some(value))
    }
}

impl From<RoutePacket> for Actions<'_> {
    fn from(value: RoutePacket) -> Self {
        Action::Route(value).into()
    }
}

impl From<SendPacket> for Actions<'_> {
    fn from(value: SendPacket) -> Self {
        Action::Send(value).into()
    }
}

#[derive(Debug, PartialEq, Default)]
pub enum TransitionToActive {
    #[default]
    Activate,
    Advertisment,
    NextARP(usize),
}

#[derive(Debug, PartialEq, Default)]
pub enum ShutdownActive {
    #[default]
    Advertisment,
    Deactivate,
    Done,
}

impl<'a> Iterator for Actions<'a> {
    type Item = Action;

    fn next(&mut self) -> Option<Self::Item> {
        use TransitionToActive::*;
        match self {
            Actions::None => None,
            Actions::ShutdownActive(parameters, shutdown) => match shutdown {
                ShutdownActive::Advertisment => {
                    *shutdown = ShutdownActive::Deactivate;
                    Some(
                        SendPacket::Advertisement(
                            Priority::SHUTDOWN,
                            parameters.advertisement_interval,
                        )
                        .into(),
                    )
                }
                ShutdownActive::Deactivate => {
                    *shutdown = ShutdownActive::Done;
                    Some(Action::Deactivate)
                }
                ShutdownActive::Done => None,
            },
            Actions::TransitionToActive(parameters, transition) => match transition {
                Activate => {
                    *transition = Advertisment;
                    Some(Action::Activate)
                }
                Advertisment => {
                    *transition = NextARP(0);
                    Some(
                        SendPacket::Advertisement(
                            parameters.priority,
                            parameters.advertisement_interval,
                        )
                        .into(),
                    )
                }
                NextARP(offset) if *offset < parameters.ipv4_addresses.len() => {
                    let next_address = parameters.ipv4_addresses[*offset];
                    *transition = NextARP(*offset + 1);
                    Some(SendPacket::GratuitousARP(parameters.mac_address(), next_address).into())
                }
                _ => None,
            },
            Actions::OneAction(action) => action.take(),
        }
    }
}
