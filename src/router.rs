use crate::{Interval, Parameters, Priority};
use pnet_base::MacAddr;
use std::net::Ipv4Addr;
use std::time::Instant;

pub struct Router {
    mac_address: MacAddr,
    parameters: Parameters,
    state: State,
}

impl Router {
    pub fn new(parameters: Parameters) -> Self {
        Self {
            mac_address: parameters.mac_address(),
            parameters,
            state: State::Initialized,
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn next_timer(&self, now: Instant) -> Instant {
        match &self.state {
            State::Initialized => now + self.parameters.advertisement_interval,
            State::Backup {
                master_down_timer, ..
            } => *master_down_timer,
            State::Master { adver_timer } => *adver_timer,
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
                            adver_timer: self.adver_timer(now),
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
                        Actions::None
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
                            adver_timer: self.adver_timer(now),
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
                        Actions::None
                    } else {
                        Actions::None
                    }
                }
                Input::Timer(now) => {
                    self.state = State::Master {
                        adver_timer: self.adver_timer(now),
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
                } if self.is_associated_address(target_ip) => Action::SendARP(ArpReply {
                    sender_mac: self.parameters.mac_address(),
                    sender_ip: target_ip,
                    target_mac: sender_mac,
                    target_ip: sender_ip,
                })
                .into(),
                Input::IpPacket(mac, ip_packet) => {
                    if mac != self.mac_address {
                        Actions::None
                    } else if self.is_associated_address(ip_packet.target_ip)
                        && (self.parameters.priority == Priority::OWNER
                            || self.parameters.accept_mode)
                    {
                        Action::AcceptPacket(ip_packet).into()
                    } else {
                        Action::ForwardPacket(ip_packet).into()
                    }
                }
                _ => Actions::None,
            },
            State::Backup {
                master_down_timer, ..
            } => match input {
                Input::Timer(now) | Input::Startup(now) if now >= *master_down_timer => {
                    self.state = State::Master {
                        adver_timer: self.adver_timer(now),
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
                    } else if priority >= self.parameters.priority || !self.parameters.preempt_mode
                    {
                        self.state = State::Backup {
                            master_down_timer: self.master_down_timer(now, master_adver_interval),
                            master_adver_interval,
                        };
                    }
                    Actions::None
                }
                _ => Actions::None,
            },
        }
    }

    fn is_associated_address(&self, ip_address: Ipv4Addr) -> bool {
        self.parameters
            .ipv4_addresses
            .iter()
            .find(|ip| **ip == ip_address)
            .is_some()
    }

    fn adver_timer(&mut self, now: Instant) -> Instant {
        now + self.parameters.advertisement_interval
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
