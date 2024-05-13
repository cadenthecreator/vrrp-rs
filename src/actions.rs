use crate::{Interval, Parameters, Priority};
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
    GratuitousARP{
        sender_mac: MacAddr,
        sender_ip: Ipv4Addr,
    },
    ReplyARP {
        sender_mac: MacAddr,
        sender_ip: Ipv4Addr,
        target_mac: MacAddr,
        target_ip: Ipv4Addr,
    },
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

impl From<Action> for Actions<'_> {
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

impl Iterator for Actions<'_> {
    type Item = Action;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Actions::None => None,
            Actions::ShutdownActive(p, shutdown) => shutdown.next_action(p),
            Actions::TransitionToActive(p, transition) => transition.next_action(p),
            Actions::OneAction(action) => action.take(),
        }
    }
}

#[derive(Debug, PartialEq, Default)]
pub enum TransitionToActive {
    #[default]
    Activate,
    Advertisment,
    NextARP(u8),
}

impl TransitionToActive {
    fn next_action(&mut self, parameters: &Parameters) -> Option<Action> {
        use TransitionToActive::*;
        match *self {
            Activate => {
                *self = Advertisment;
                Some(Action::Activate)
            }
            Advertisment => {
                *self = NextARP(0);
                Some(
                    SendPacket::Advertisement(
                        parameters.priority,
                        parameters.advertisement_interval,
                    )
                    .into(),
                )
            }
            NextARP(offset)
                if offset <= u8::MAX && offset < parameters.ipv4_addresses.len() as u8 =>
            {
                let next_address = parameters.ipv4_addresses[offset as usize];
                *self = NextARP(offset + 1);
                Some(SendPacket::GratuitousARP { sender_mac: parameters.mac_address(), sender_ip: next_address }.into())
            }
            NextARP(_) => None,
        }
    }
}

#[derive(Debug, PartialEq, Default)]
pub enum ShutdownActive {
    #[default]
    Advertisment,
    Deactivate,
    Done,
}

impl ShutdownActive {
    fn next_action(&mut self, parameters: &Parameters) -> Option<Action> {
        match *self {
            ShutdownActive::Advertisment => {
                *self = ShutdownActive::Deactivate;
                Some(
                    SendPacket::Advertisement(
                        Priority::SHUTDOWN,
                        parameters.advertisement_interval,
                    )
                    .into(),
                )
            }
            ShutdownActive::Deactivate => {
                *self = ShutdownActive::Done;
                Some(Action::Deactivate)
            }
            ShutdownActive::Done => None,
        }
    }
}
