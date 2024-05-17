use crate::actions::{Actions, RoutePacket, SendPacket};
use crate::input::ReceivedPacket;
use crate::{Action, Command, Input, Interval, Parameters, Priority};
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

    pub fn handle_input(
        &mut self,
        now: Instant,
        input: Input,
    ) -> impl Iterator<Item = Action> + '_ {
        match &self.state {
            State::Initialized => match input {
                Input::Command(Command::Startup) => self.startup(now),
                Input::Command(Command::Shutdown) => Actions::None,
                Input::Timer => Actions::None,
                Input::Packet(ReceivedPacket::ShutdownAdvertisement { .. }) => Actions::None,
                Input::Packet(ReceivedPacket::Advertisement { .. }) => Actions::None,
                Input::Packet(ReceivedPacket::RequestARP { .. }) => Actions::None,
                Input::Packet(ReceivedPacket::IP { .. }) => RoutePacket::Reject.into(),
            },
            State::Active { adver_timer } => match input {
                Input::Command(Command::Shutdown) => self.shutdown_active(),
                Input::Command(Command::Startup) => Actions::None,
                Input::Packet(ReceivedPacket::ShutdownAdvertisement { .. }) => {
                    self.send_advertisment(now)
                }
                Input::Packet(ReceivedPacket::Advertisement {
                    priority,
                    active_adver_interval,
                }) => self.active_advertisement(now, priority, active_adver_interval),
                Input::Timer if now >= *adver_timer => self.send_advertisment(now),
                Input::Timer => Actions::None,
                Input::Packet(ReceivedPacket::RequestARP {
                    sender_ip,
                    sender_mac,
                    target_ip,
                }) if self.is_associated_address(target_ip) => SendPacket::ReplyARP {
                    sender_mac: self.mac_address,
                    sender_ip: target_ip,
                    target_mac: sender_mac,
                    target_ip: sender_ip,
                }
                .into(),
                Input::Packet(ReceivedPacket::RequestARP { .. }) => Actions::None,
                Input::Packet(ReceivedPacket::IP {
                    target_mac,
                    target_ip,
                }) => self.route_ip_packet(target_mac, target_ip),
            },
            State::Backup {
                active_down_timer, ..
            } => match input {
                Input::Timer if now >= *active_down_timer => self.transition_to_active(now),
                Input::Timer => Actions::None,
                Input::Command(Command::Startup) => self.transition_to_active(now),
                Input::Command(Command::Shutdown) => self.shutdown_backup(),
                Input::Packet(ReceivedPacket::ShutdownAdvertisement {
                    active_adver_interval,
                }) => self.update_active_diown_timer_for_shutdown(now, active_adver_interval),
                Input::Packet(ReceivedPacket::Advertisement {
                    priority,
                    active_adver_interval,
                }) => self.update_active_down_timer(now, priority, active_adver_interval),
                Input::Packet(ReceivedPacket::IP { .. }) => RoutePacket::Reject.into(),
                Input::Packet(ReceivedPacket::RequestARP { .. }) => Actions::None,
            },
        }
    }

    fn startup(&mut self, now: Instant) -> Actions {
        if self.parameters.priority == Priority::OWNER {
            self.transition_to_active(now)
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

    fn transition_to_active(&mut self, now: Instant) -> Actions {
        self.state = State::Active {
            adver_timer: self.adver_timer(now),
        };
        Actions::TransitionToActive(&self.parameters, Default::default())
    }

    fn send_advertisment(&mut self, now: Instant) -> Actions {
        self.state = State::Active {
            adver_timer: self.adver_timer(now),
        };
        SendPacket::Advertisement(&self.parameters).into()
    }

    fn active_advertisement(
        &mut self,
        now: Instant,
        priority: Priority,
        active_adver_interval: Interval,
    ) -> Actions {
        if priority == Priority::SHUTDOWN {
            self.send_advertisment(now)
        } else if priority > self.parameters.priority {
            self.deactivate(now, active_adver_interval)
        } else {
            Actions::None
        }
    }

    fn deactivate(&mut self, now: Instant, active_adver_interval: Interval) -> Actions {
        self.state = State::Backup {
            active_down_timer: self.active_down_timer(now, active_adver_interval),
            active_adver_interval,
        };
        Action::Deactivate.into()
    }

    fn update_active_down_timer(
        &mut self,
        now: Instant,
        priority: Priority,
        active_adver_interval: Interval,
    ) -> Actions {
        if priority >= self.parameters.priority || !self.parameters.preempt_mode {
            self.state = State::Backup {
                active_down_timer: self.active_down_timer(now, active_adver_interval),
                active_adver_interval,
            };
        }
        Actions::None
    }

    fn update_active_diown_timer_for_shutdown(
        &mut self,
        now: Instant,
        active_adver_interval: Interval,
    ) -> Actions {
        self.state = State::Backup {
            active_down_timer: self.active_down_timer_for_shutdown(now, active_adver_interval),
            active_adver_interval,
        };
        Actions::None
    }

    fn route_ip_packet(&mut self, target_mac: MacAddr, target_ip: Ipv4Addr) -> Actions {
        if target_mac != self.mac_address {
            Actions::None
        } else if self.is_associated_address(target_ip)
            && (self.parameters.priority == Priority::OWNER || self.parameters.accept_mode)
        {
            RoutePacket::Accept.into()
        } else {
            RoutePacket::Forward.into()
        }
    }

    fn shutdown_active(&mut self) -> Actions {
        self.state = State::Initialized;
        Actions::ShutdownActive(&self.parameters, Default::default())
    }

    fn shutdown_backup(&mut self) -> Actions {
        self.state = State::Initialized;
        Actions::None
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

    fn active_down_timer(&self, now: Instant, active_adver_interval: Interval) -> Instant {
        now + self.parameters.active_down_interval(active_adver_interval)
    }

    fn active_down_timer_for_shutdown(
        &self,
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
