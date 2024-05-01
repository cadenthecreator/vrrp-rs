use crate::{Interval, Parameters, Priority};
use pnet_base::MacAddr;
use std::net::Ipv4Addr;
use std::time::Instant;

#[derive(Debug, PartialEq)]
pub enum Input {
    Advertisement(Instant, Priority, Interval),
    Startup(Instant),
    Timer(Instant),
    Shutdown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    WaitForInput,
    SendAdvertisement(Priority,Interval),
    BroadcastGratuitousARP(MacAddr, Ipv4Addr),
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

    pub fn handle_input(&mut self, input: Input) -> impl Iterator<Item = Action> + '_ {
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
                        Actions::WaitForInput
                    }
                }
                _ => Actions::None,
            },
            State::Master { .. } => match input {
                Input::Shutdown => {
                    self.state = State::Initialized;
                    Actions::SendAdvertisement {
                        priority: Priority::SHUTDOWN,
                        master_adver_interval: self.parameters.advertisement_interval,
                    }
                }
                    Input::Advertisement(now, priority, master_adver_interval) => {
                        if priority == Priority::SHUTDOWN {
                            self.state = State::Master {
                                adver_timer: now + self.parameters.advertisement_interval,
                            };
                            Actions::SendAdvertisement {
                                priority: self.parameters.priority,
                                master_adver_interval: self.parameters.advertisement_interval,
                            }
                        } else if priority > self.parameters.priority {
                            self.state = State::Backup {
                                master_down_timer: self.master_down_timer(now,master_adver_interval),
                                master_adver_interval,
                            };
                            Actions::WaitForInput
                        } else {
                            Actions::WaitForInput
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
                    Actions::WaitForInput
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
    WaitForInput,
    SendAdvertisement {
        priority: Priority,
        master_adver_interval: Interval,
    },
    TransitionToMaster {
        parameters: &'a Parameters,
        next_arp_offset: Option<usize>,
    },
    None,
}

impl Iterator for Actions<'_> {
    type Item = Action;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Actions::WaitForInput => {
                *self = Actions::None;
                Some(Action::WaitForInput)
            }
            Actions::SendAdvertisement { priority, master_adver_interval} => {
                let priority = *priority;
                let master_adver_interval = *master_adver_interval;
                *self = Actions::None;
                Some(Action::SendAdvertisement(priority, master_adver_interval))
            }
            Actions::None => None,
            Actions::TransitionToMaster {
                parameters,
                next_arp_offset,
            } => match *next_arp_offset {
                None => {
                    *next_arp_offset = Some(0);
                    Some(Action::SendAdvertisement(parameters.priority,parameters.advertisement_interval))
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
        }
    }
}
