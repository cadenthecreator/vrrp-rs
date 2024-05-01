use crate::{Interval, Parameters, Priority};
use pnet_base::MacAddr;
use std::net::Ipv4Addr;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArpReply {
    pub sender_mac: MacAddr,
    pub sender_ip: Ipv4Addr,
    pub target_mac: MacAddr,
    pub target_ip: Ipv4Addr,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IpPacket<'a> {
    pub sender_ip: Ipv4Addr,
    pub target_ip: Ipv4Addr,
    pub data: &'a [u8],
}
#[derive(Debug, PartialEq)]
pub enum Input<'a> {
    Advertisement(Instant, Priority, Interval),
    ARP {
        sender_mac: MacAddr,
        sender_ip: Ipv4Addr,
        target_ip: Ipv4Addr,
    },
    Startup(Instant),
    Timer(Instant),
    IpPacket(MacAddr, IpPacket<'a>),
    Shutdown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action<'a> {
    ForwardPacket(IpPacket<'a>),
    WaitForInput,
    SendAdvertisement(Priority, Interval),
    BroadcastGratuitousARP(MacAddr, Ipv4Addr),
    AcceptPacket(IpPacket<'a>),
    SendARP(ArpReply),
}

#[derive(Debug, PartialEq)]
pub enum State {
    Initialized,
    Backup {
        master_down_timer: Instant,
        master_adver_interval: Interval,
    },
    Master {
        adver_timer: Instant,
    },
}

pub struct Router {
    parameters: Parameters,
    state: State,
}

impl Router {
    pub fn new(parameters: Parameters) -> Self {
        Self {
            parameters,
            state: State::Initialized,
        }
    }

    pub fn handle_input<'a>(
        &'a mut self,
        input: Input<'a>,
    ) -> impl Iterator<Item = Action<'a>> + 'a {
        match &self.state {
            State::Initialized => match input {
                Input::Startup(now) => {
                    let priority = self.parameters.priority;
                    if priority == Priority::OWNER {
                        self.state = State::Master {
                            adver_timer: now + self.parameters.advertisement_interval,
                        };
                        Actions::TransitionToMaster {
                            parameters: &self.parameters,
                            next_arp_offset: None,
                        }
                    } else {
                        let master_adver_interval = self.parameters.advertisement_interval;
                        let master_down_timer = self.master_down_timer(now, master_adver_interval);
                        self.state = State::Backup {
                            master_adver_interval,
                            master_down_timer,
                        };
                        Action::WaitForInput.into()
                    }
                }
                _ => Actions::None,
            },
            State::Master { .. } => match input {
                Input::Shutdown => {
                    self.state = State::Initialized;
                    Action::SendAdvertisement(
                        Priority::SHUTDOWN,
                        self.parameters.advertisement_interval,
                    )
                    .into()
                }
                Input::Advertisement(now, priority, master_adver_interval) => {
                    if priority == Priority::SHUTDOWN {
                        self.state = State::Master {
                            adver_timer: now + self.parameters.advertisement_interval,
                        };
                        Action::SendAdvertisement(
                            self.parameters.priority,
                            self.parameters.advertisement_interval,
                        )
                        .into()
                    } else if priority > self.parameters.priority {
                        self.state = State::Backup {
                            master_down_timer: self.master_down_timer(now, master_adver_interval),
                            master_adver_interval,
                        };
                        Action::WaitForInput.into()
                    } else {
                        Action::WaitForInput.into()
                    }
                }
                Input::Timer(now) => {
                    self.state = State::Master {
                        adver_timer: now + self.parameters.advertisement_interval,
                    };
                    Action::SendAdvertisement(
                        self.parameters.priority,
                        self.parameters.advertisement_interval,
                    )
                    .into()
                }
                Input::ARP {
                    sender_ip,
                    sender_mac,
                    target_ip,
                } if self
                    .parameters
                    .ip_addresses
                    .iter()
                    .find(|ip| **ip == target_ip)
                    .is_some() =>
                {
                    Action::SendARP(ArpReply {
                        sender_mac: self.parameters.mac_address,
                        sender_ip: target_ip,
                        target_mac: sender_mac,
                        target_ip: sender_ip,
                    })
                    .into()
                }
                Input::IpPacket(mac, ip_packet) => {
                    if self
                        .parameters
                        .ip_addresses
                        .iter()
                        .find(|ip| **ip == ip_packet.target_ip)
                        .is_some()
                        && mac == self.parameters.mac_address
                    {
                        Action::AcceptPacket(ip_packet).into()
                    } else if mac == self.parameters.mac_address {
                        Action::ForwardPacket(ip_packet).into()
                    } else {
                        Action::WaitForInput.into()
                    }
                }
                _ => Actions::None,
            },
            State::Backup {
                master_down_timer, ..
            } => match input {
                Input::Timer(now) | Input::Startup(now) if now >= *master_down_timer => {
                    self.state = State::Master {
                        adver_timer: now + self.parameters.advertisement_interval,
                    };
                    Actions::TransitionToMaster {
                        parameters: &self.parameters,
                        next_arp_offset: None,
                    }
                }
                Input::Shutdown => {
                    self.state = State::Initialized;
                    Actions::None
                }
                Input::Advertisement(now, priority, master_adver_interval) => {
                    if priority == Priority::SHUTDOWN {
                        self.state = State::Backup {
                            master_down_timer: self
                                .master_down_timer_for_shutdown(now, master_adver_interval),
                            master_adver_interval,
                        }
                    } else if priority >= self.parameters.priority {
                        self.state = State::Backup {
                            master_down_timer: self.master_down_timer(now, master_adver_interval),
                            master_adver_interval,
                        };
                    }
                    Action::WaitForInput.into()
                }
                _ => Actions::None,
            },
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    fn master_down_timer(&mut self, now: Instant, master_adver_interval: Interval) -> Instant {
        now + self.parameters.master_down_interval(master_adver_interval)
    }

    fn master_down_timer_for_shutdown(
        &mut self,
        now: Instant,
        master_adver_interval: Interval,
    ) -> Instant {
        now + self.parameters.skew_time(master_adver_interval)
    }
}

#[derive(Debug, PartialEq)]
enum Actions<'a> {
    TransitionToMaster {
        parameters: &'a Parameters,
        next_arp_offset: Option<usize>,
    },
    OneAction(Action<'a>),
    None,
}
impl<'a> From<Action<'a>> for Actions<'a> {
    fn from(value: Action<'a>) -> Self {
        Actions::OneAction(value)
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
                Some(offset) if offset < parameters.ip_addresses.len() => {
                    let next_address = parameters.ip_addresses[offset];
                    *next_arp_offset = Some(offset + 1);
                    Some(Action::BroadcastGratuitousARP(
                        parameters.mac_address,
                        next_address,
                    ))
                }
                _ => None,
            },
            Actions::OneAction(action) => {
                let action = *action;
                *self = Actions::None;
                Some(action)
            }
        }
    }
}
