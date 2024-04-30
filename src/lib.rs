mod interval;
mod priority;

use std::net::Ipv4Addr;
use std::time::Instant;

use pnet_base::MacAddr;

pub use interval::Interval;
pub use priority::Priority;

#[derive(Clone, Debug, PartialEq)]
pub struct RouterParameters {
    mac_address: MacAddr,
    ip_addresses: Vec<Ipv4Addr>,
    priority: Priority,
    advertisement_interval: Interval,
}

impl RouterParameters {
    pub fn ipv4(&self, index: usize) -> Ipv4Addr {
        self.ip_addresses[index]
    }
}

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
    SendAdvertisement(Priority),
    BroadcastGratuitousARP(MacAddr, Ipv4Addr),
}

#[derive(Debug, PartialEq)]
pub enum State {
    Initialized,
    Backup { master_down_timer: Instant, master_adver_interval: Interval },
    Master { adver_timer: Instant },
}

pub struct VirtualRouter {
    parameters: RouterParameters,
    state: State,
}

impl VirtualRouter {
    pub fn new(parameters: RouterParameters) -> Self {
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
                            sent_announcement: false,
                            next_arp_offset: 0,
                        }
                    } else {
                        let master_adver_interval = self.parameters.advertisement_interval;
                        let master_down_timer = now + self.parameters.priority.master_down_interval(master_adver_interval);
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
                    }
                }
                _ => Actions::None,
            },
            State::Backup { master_down_timer, .. } => match input {
                Input::Timer(now) | Input::Startup(now) if now >= *master_down_timer => {
                    self.state = State::Master {
                        adver_timer: now + self.parameters.advertisement_interval,
                    };
                    Actions::TransitionToMaster {
                        parameters: &self.parameters,
                        sent_announcement: false,
                        next_arp_offset: 0,
                    }
                }
                Input::Shutdown => {
                    self.state = State::Initialized;
                    Actions::None
                }
                Input::Advertisement(now, priority, master_adver_interval) => {
                    if priority == Priority::SHUTDOWN {
                        self.state = State::Backup {
                            master_down_timer: self.master_down_timer_for_shutdown(now, master_adver_interval),
                            master_adver_interval,
                        }
                    } else {
                        if priority.0 >= self.parameters.priority.0 {
                            self.state = State::Backup {
                                master_down_timer: self.master_down_timer(now, master_adver_interval),
                                master_adver_interval,
                            };
                        }
                    }
                    Actions::WaitForInput
                }
                _ => Actions::None,
            },
        }
    }

    fn master_down_timer(&mut self, now: Instant, master_adver_interval: Interval) -> Instant {
        self.parameters.priority.master_down_timer(now, master_adver_interval)
    }

    fn master_down_timer_for_shutdown(&mut self, now: Instant, master_adver_interval: Interval) -> Instant {
        now + self.parameters.priority.skew_time(master_adver_interval)
    }

    pub fn state(&self) -> &State {
        &self.state
    }
}

#[derive(Debug, PartialEq)]
enum Actions<'a> {
    WaitForInput,
    SendAdvertisement {
        priority: Priority,
    },
    TransitionToMaster {
        parameters: &'a RouterParameters,
        sent_announcement: bool,
        next_arp_offset: usize,
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
            Actions::SendAdvertisement { priority } => {
                let priority = *priority;
                *self = Actions::None;
                Some(Action::SendAdvertisement(priority))
            }
            Actions::None => None,
            Actions::TransitionToMaster {
                parameters,
                sent_announcement,
                next_arp_offset,
            } => {
                if !*sent_announcement {
                    *sent_announcement = true;
                    Some(Action::SendAdvertisement(parameters.priority))
                } else if *next_arp_offset < parameters.ip_addresses.len() {
                    let next_address = parameters.ip_addresses[*next_arp_offset];
                    *next_arp_offset += 1;
                    Some(Action::BroadcastGratuitousARP(
                        parameters.mac_address,
                        next_address,
                    ))
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    use pretty_assertions::assert_eq;

    fn startup_with_priority(priority: Priority) -> (VirtualRouter, RouterParameters, Instant) {
        let (mut router, parameters) = router_with_priority(priority);
        let now = Instant::now();
        let _ = router.handle_input(Input::Startup(now));
        (router, parameters, now)
    }

    fn router_with_priority(priority: Priority) -> (VirtualRouter, RouterParameters) {
        let ip_1 = Ipv4Addr::new(1, 1, 1, 1);
        let ip_2 = Ipv4Addr::new(2, 2, 2, 2);
        let ip_addresses = vec![ip_1, ip_2];
        let advertisement_interval = Interval::from_secs(1);
        let mac_address = MacAddr::new(1, 1, 1, 1, 1, 1);

        let parameters = RouterParameters {
            mac_address,
            ip_addresses,
            advertisement_interval,
            priority,
        };

        let router = VirtualRouter::new(parameters.clone());

        (router, parameters)
    }

    #[test]
    fn startup() {
        let (mut router, p) = router_with_priority(Priority::default());

        assert_eq!(
            *router.state(),
            State::Initialized,
            "all routers should begin in the initialized state"
        );

        let now = Instant::now();
        assert_eq!(
            router.handle_input(Input::Startup(now)).collect::<Vec<_>>(),
            vec![Action::WaitForInput]
        );
        assert_eq!(
            *router.state(),
            State::Backup {
                master_down_timer: now
                    + 3 * p.advertisement_interval + ((256 - 100) * p.advertisement_interval / 256),
                master_adver_interval: p.advertisement_interval,
            },
            "after startup, an un-owned router should transition to the Backup state"
        );
    }

    #[test]
    fn startup_address_owner() {
        let (mut router, p) = router_with_priority(Priority::OWNER);

        assert_eq!(
            *router.state(),
            State::Initialized,
            "all routers should begin in the initialized state"
        );

        // On Startup
        // If the router owns the IP address(es) associated with the virtual router
        let now = Instant::now();
        let actions = router.handle_input(Input::Startup(now)).collect::<Vec<_>>();
        assert_eq!(
            actions[0],
            Action::SendAdvertisement(Priority::OWNER),
            "it should Send an ADVERTISEMENT"
        );
        assert_eq!(vec![actions[1], actions[2]], vec![Action::BroadcastGratuitousARP(p.mac_address, p.ipv4(0)), Action::BroadcastGratuitousARP(p.mac_address, p.ipv4(1))], "for each IP address associated with the virtual router, it should broadcast a gratuitous ARP request containing the virtual router MAC address");
        assert_eq!(
            *router.state(),
            State::Master {
                adver_timer: now + p.advertisement_interval
            },
            "after startup, an owned router should transition to the Master state"
        );
    }

    #[test]
    fn backup_master_down_timer_fires() {
        let (mut router, p, now) = startup_with_priority(Priority::default());

        let now = p.priority.master_down_timer(now, p.advertisement_interval);
        let actions = router.handle_input(Input::Timer(now)).collect::<Vec<_>>();
        assert_eq!(
            actions[0],
            Action::SendAdvertisement(Priority(100)),
            "it should Send an ADVERTISEMENT"
        );
        assert_eq!(vec![actions[1], actions[2]], vec![Action::BroadcastGratuitousARP(p.mac_address, p.ipv4(0)), Action::BroadcastGratuitousARP(p.mac_address, p.ipv4(1))], "for each IP address associated with the virtual router, it should broadcast a gratuitous ARP request containing the virtual router MAC address");
        assert_eq!(*router.state(), State::Master { adver_timer: now + p.advertisement_interval }, "it should transition to the Master state and set the Adver_Timer to Advertisement_Interval");
    }

    #[test]
    fn backup_shutdown() {
        let (mut router, _, _) = startup_with_priority(Priority::default());

        let actions = router.handle_input(Input::Shutdown).collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![],
            "router should be doing nothing but it is not"
        );
        assert_eq!(
            *router.state(),
            State::Initialized,
            "all routers should end in the initialized state"
        );
    }

    #[test]
    fn master_shutdown() {
        let (mut router, _, _) = startup_with_priority(Priority::OWNER);

        let actions = router.handle_input(Input::Shutdown).collect::<Vec<_>>();

        assert_eq!(actions, vec![Action::SendAdvertisement(Priority::SHUTDOWN)]);
        assert_eq!(
            *router.state(),
            State::Initialized,
            "all routers should end in the initialized state"
        );
    }

    #[test]
    fn backup_receive_shutdown_advertisement() {
        let (mut router, p, now) = startup_with_priority(Priority::default());

        let expected_master_adver_interval = Interval::from_secs(10);
        let actions = router
            .handle_input(Input::Advertisement(now, Priority::SHUTDOWN, expected_master_adver_interval))
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![Action::WaitForInput]);
        assert_eq!(
            *router.state(),
            State::Backup {
                master_down_timer: now + (((256 - p.priority.0 as u32) * expected_master_adver_interval) / 256),
                master_adver_interval: expected_master_adver_interval,
            }
        );

    }

    #[test]
    fn backup_receive_greater_priority_advertisement() {
        let (mut router, p, now) = startup_with_priority(Priority::default());

        let expected_master_adver_interval = Interval::from_secs(5);
        let actions = router
            .handle_input(Input::Advertisement(now, Priority(101), expected_master_adver_interval))
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![Action::WaitForInput]);
        assert_eq!(
            *router.state(),
            State::Backup {
                master_down_timer: now + (3 * expected_master_adver_interval) + (((256 - p.priority.0 as u32) * expected_master_adver_interval) / 256),
                master_adver_interval: expected_master_adver_interval,
            }
        );

    }
}
