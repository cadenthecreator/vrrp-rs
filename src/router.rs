use crate::actions::{Actions, RoutePacket, SendPacket};
use crate::input::Packet;
use crate::{Action, ArpReply, Command, Input, Interval, Parameters, Priority};
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
                active_down_timer, ..
            } => *active_down_timer,
            State::Active { adver_timer } => *adver_timer,
        }
    }

    pub fn handle_input<'a>(
        &'a mut self,
        now: Instant,
        input: Input,
    ) -> impl Iterator<Item = Action> + 'a {
        match &self.state {
            State::Initialized => match input {
                Input::Command(Command::Startup) => {
                    let priority = self.parameters.priority;
                    if priority == Priority::OWNER {
                        self.state = State::Active {
                            adver_timer: self.adver_timer(now),
                        };
                        Actions::TransitionToActive(&self.parameters, Default::default())
                    } else {
                        let active_adver_interval = self.parameters.advertisement_interval;
                        let active_down_timer = self.active_down_timer(now, active_adver_interval);
                        self.state = State::Backup {
                            active_adver_interval,
                            active_down_timer,
                        };
                        Actions::None
                    }
                }
                _ => Actions::None,
            },
            State::Active { adver_timer } => match input {
                Input::Command(Command::Shutdown) => {
                    self.state = State::Initialized;
                    Actions::ShutdownActive(&self.parameters, Default::default())
                }
                Input::Packet(Packet::Advertisement(priority, active_adver_interval)) => {
                    if priority == Priority::SHUTDOWN {
                        self.state = State::Active {
                            adver_timer: self.adver_timer(now),
                        };
                        SendPacket::Advertisement(
                            self.parameters.priority,
                            self.parameters.advertisement_interval,
                        )
                        .into()
                    } else if priority > self.parameters.priority {
                        self.state = State::Backup {
                            active_down_timer: self.active_down_timer(now, active_adver_interval),
                            active_adver_interval,
                        };
                        Action::Deactivate.into()
                    } else {
                        Actions::None
                    }
                }
                Input::Timer if now >= *adver_timer => {
                    self.state = State::Active {
                        adver_timer: self.adver_timer(now),
                    };
                    SendPacket::Advertisement(
                        self.parameters.priority,
                        self.parameters.advertisement_interval,
                    )
                    .into()
                }
                Input::Packet(Packet::ARP {
                    sender_ip,
                    sender_mac,
                    target_ip,
                }) if self.is_associated_address(target_ip) => SendPacket::ARP(ArpReply {
                    sender_mac: self.mac_address,
                    sender_ip: target_ip,
                    target_mac: sender_mac,
                    target_ip: sender_ip,
                })
                .into(),
                Input::Packet(Packet::IpPacket(mac, ip_packet)) => {
                    if mac != self.mac_address {
                        Actions::None
                    } else if self.is_associated_address(ip_packet.target_ip)
                        && (self.parameters.priority == Priority::OWNER
                            || self.parameters.accept_mode)
                    {
                        RoutePacket::Accept.into()
                    } else {
                        RoutePacket::Forward.into()
                    }
                }
                _ => Actions::None,
            },
            State::Backup {
                active_down_timer, ..
            } => match input {
                Input::Timer | Input::Command(Command::Startup) if now >= *active_down_timer => {
                    self.state = State::Active {
                        adver_timer: self.adver_timer(now),
                    };
                    Actions::TransitionToActive(&self.parameters, Default::default())
                }
                Input::Command(Command::Shutdown) => {
                    self.state = State::Initialized;
                    Actions::None
                }
                Input::Packet(Packet::Advertisement(priority, active_adver_interval)) => {
                    if priority == Priority::SHUTDOWN {
                        self.state = State::Backup {
                            active_down_timer: self
                                .active_down_timer_for_shutdown(now, active_adver_interval),
                            active_adver_interval,
                        }
                    } else if priority >= self.parameters.priority || !self.parameters.preempt_mode
                    {
                        self.state = State::Backup {
                            active_down_timer: self.active_down_timer(now, active_adver_interval),
                            active_adver_interval,
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

    fn active_down_timer(&mut self, now: Instant, active_adver_interval: Interval) -> Instant {
        now + self.parameters.active_down_interval(active_adver_interval)
    }

    fn active_down_timer_for_shutdown(
        &mut self,
        now: Instant,
        active_adver_interval: Interval,
    ) -> Instant {
        now + self.parameters.skew_time(active_adver_interval)
    }
}

#[derive(Debug, PartialEq)]
pub enum State {
    Initialized,
    Backup {
        active_down_timer: Instant,
        active_adver_interval: Interval,
    },
    Active {
        adver_timer: Instant,
    },
}
