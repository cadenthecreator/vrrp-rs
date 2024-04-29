use std::net::{Ipv4Addr};
use std::time::{Duration, Instant};
use pnet_base::MacAddr;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Priority(pub u8);

impl Priority {
    const  OWNER : Priority = Priority(255);
}

impl Default for Priority {
    fn default() -> Self {
        Priority(100)
    }
}

impl Into<Duration> for Priority {
    fn into(self) -> Duration {
        Duration::from_secs(self.0 as u64)
    }
}

#[derive(Debug, PartialEq)]
pub struct RouterParameters {
    mac_address: MacAddr,
    ip_addresses: Vec<Ipv4Addr>,
    priority: Priority,
    advertisement_interval: Duration,
}

#[derive(Debug, PartialEq)]
pub enum Input {
    Startup(Instant),
    Timer(Instant),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    WaitForInput,
    SendAdvertisement,
    BroadcastGratuitousARP(MacAddr, Ipv4Addr),
}

#[derive(Debug, PartialEq)]
pub enum State {
    Initialized,
    Backup  { master_down_timer: Instant },
    Master,
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

    pub fn handle_input(&mut self, input: Input) -> impl Iterator<Item=Action> + '_ {
        match &self.state {
            State::Initialized => {
                match input {
                    Input::Startup(now) => {
                        let priority = self.parameters.priority;
                        if priority == Priority::OWNER {
                            self.state = State::Master;
                            Actions::TransitionInitializeToMaster {
                                parameters: &self.parameters,
                                sent_announcement: false,
                                next_arp_offset: 0
                            }
                        } else {
                            let skew_time = Duration::from_secs( (256 - priority.0 as u64) / 256 );
                            let master_down_interval = 3 * self.parameters.advertisement_interval + skew_time;
                            self.state = State::Backup { master_down_timer: now + master_down_interval };
                            Actions::WaitForInput
                        }

                    }
                    Input::Timer(_) => { Actions::None }
                }
            }
            State::Master  => {
                Actions::None
            },
            State::Backup { .. } => {
                Actions::None
            }
        }
    }

    pub fn state(&self) -> &State {
        &self.state
    }
}

#[derive(Debug, PartialEq)]
enum Actions<'a> {
    WaitForInput,
    TransitionInitializeToMaster { parameters: &'a RouterParameters, sent_announcement: bool, next_arp_offset: usize },
    None,
}

impl Iterator for Actions<'_> {
    type Item = Action;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Actions::WaitForInput => { *self = Actions::None; Some(Action::WaitForInput) }
            Actions::None => None,
            Actions::TransitionInitializeToMaster { parameters, sent_announcement, next_arp_offset} => {
                if !*sent_announcement {
                    *sent_announcement = true;
                    Some(Action::SendAdvertisement)
                } else if *next_arp_offset < parameters.ip_addresses.len() {
                    let next_address = parameters.ip_addresses[*next_arp_offset];
                    *next_arp_offset += 1;
                    Some(Action::BroadcastGratuitousARP(parameters.mac_address, next_address))
                } else {
                    None
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;
    use super::*;

    use pretty_assertions::assert_eq;

    #[test]
    fn startup() {
        let ip_1 = Ipv4Addr::new(1, 1, 1, 1);
        let ip_2 = Ipv4Addr::new(2, 2, 2, 2);
        let ip_addresses = vec![ip_1, ip_2];
        let advertisement_interval = Duration::from_secs(1);
        let mac_address = MacAddr::new(1, 1, 1, 1, 1, 1);
        let parameters = RouterParameters { mac_address, ip_addresses, advertisement_interval, priority: Priority::default() };

        let mut router = VirtualRouter::new(parameters);
        assert_eq!(*router.state(), State::Initialized, "all routers should begin in the initialized state");

        let now = Instant::now();
        assert_eq!(router.handle_input(Input::Startup(now)).collect::<Vec<_>>(), vec![Action::WaitForInput]);
        assert_eq!(*router.state(), State::Backup { master_down_timer: now + 3 * advertisement_interval + Duration::from_secs((256 - 100)/ 256) }, "after startup, an un-owned router should transition to the Backup state");
    }

    #[test]
    fn startup_address_owner() {        let ip_1 = Ipv4Addr::new(1, 1, 1, 1);
        let ip_2 = Ipv4Addr::new(2, 2, 2, 2);
        let ip_addresses = vec![ip_1, ip_2];
        let advertisement_interval = Duration::from_secs(2);
        let mac_address = MacAddr::new(1, 1, 1, 1, 1, 1);
        let parameters = RouterParameters { mac_address, ip_addresses, advertisement_interval, priority: Priority::OWNER };

        let mut router = VirtualRouter::new(parameters);
        assert_eq!(*router.state(), State::Initialized, "all routers should begin in the initialized state");

        // On Startup
        // If the router owns the IP address(es) associated with the virtual router
        let now = Instant::now();
        let actions = router.handle_input(Input::Startup(now)).collect::<Vec<_>>();
        assert_eq!(actions[0], Action::SendAdvertisement, "it should Send an ADVERTISEMENT");
        assert_eq!(vec![actions[1], actions[2]], vec![Action::BroadcastGratuitousARP(mac_address, ip_1), Action::BroadcastGratuitousARP(mac_address, ip_2)], "for each IP address associated with the virtual router, it should broadcast a gratuitous ARP request containing the virtual router MAC address");
        assert_eq!(*router.state(), State::Master, "after startup, an owned router should transition to the Master state");
    }

    #[test]
    fn backup_master_down_timer_fires() {

    }
}
