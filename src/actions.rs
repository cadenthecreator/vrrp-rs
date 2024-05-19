use crate::send::SendPacket;
use crate::Parameters;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action<'a> {
    Activate,
    Deactivate,
    Send(SendPacket<'a>),
    Route(RoutePacket),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RoutePacket {
    Reject,
    Accept,
    Forward,
}

impl From<RoutePacket> for Action<'_> {
    fn from(value: RoutePacket) -> Self {
        Self::Route(value)
    }
}

impl<'a> From<SendPacket<'a>> for Action<'a> {
    fn from(value: SendPacket<'a>) -> Self {
        Self::Send(value)
    }
}

#[derive(Debug, PartialEq)]
pub enum Actions<'a> {
    TransitionToActive(&'a Parameters, TransitionToActive),
    ShutdownActive(&'a Parameters, ShutdownActive),
    OneAction(Option<Action<'a>>),
    None,
}

impl<'a> From<Action<'a>> for Actions<'a> {
    fn from(value: Action<'a>) -> Self {
        Actions::OneAction(Some(value))
    }
}

impl From<RoutePacket> for Actions<'_> {
    fn from(value: RoutePacket) -> Self {
        Action::Route(value).into()
    }
}

impl<'a> From<SendPacket<'a>> for Actions<'a> {
    fn from(value: SendPacket<'a>) -> Self {
        Action::Send(value).into()
    }
}

impl<'a> Iterator for Actions<'a> {
    type Item = Action<'a>;

    fn next(&mut self) -> Option<Action<'a>> {
        match self {
            Actions::None => None,
            Actions::ShutdownActive(p, shutdown) => shutdown.next_action(*p),
            Actions::TransitionToActive(p, transition) => transition.next_action(*p),
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
    fn next_action<'a, 'b>(&'a mut self, parameters: &'b Parameters) -> Option<Action<'b>> {
        use TransitionToActive::*;
        match *self {
            Activate => {
                *self = Advertisment;
                Some(Action::Activate)
            }
            Advertisment => {
                *self = NextARP(0);
                Some(SendPacket::Advertisement(&parameters).into())
            }
            NextARP(offset) => parameters
                .virtual_addresses
                .get(offset)
                .map(|next_address| {
                    *self = NextARP(offset + 1);
                    SendPacket::GratuitousARP {
                        sender_mac: parameters.mac_address(),
                        sender_ip: next_address,
                    }
                    .into()
                }),
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
    fn next_action<'a, 'b>(&'a mut self, parameters: &'b Parameters) -> Option<Action<'b>> {
        match *self {
            ShutdownActive::Advertisment => {
                *self = ShutdownActive::Deactivate;
                Some(SendPacket::ShutdownAdvertisement(parameters).into())
            }
            ShutdownActive::Deactivate => {
                *self = ShutdownActive::Done;
                Some(Action::Deactivate)
            }
            ShutdownActive::Done => None,
        }
    }
}
