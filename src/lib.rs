mod interval;
mod parameters;
mod priority;
mod router;

use std::net::Ipv4Addr;

use pnet_base::MacAddr;

pub use interval::Interval;
pub use parameters::RouterParameters;
pub use priority::Priority;
pub use router::{Action, Input, State, VirtualRouter};

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
                    + 3 * p.advertisement_interval
                    + ((256 - 100) * p.advertisement_interval / 256),
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
            Action::SendAdvertisement(Priority::new(100)),
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
            .handle_input(Input::Advertisement(
                now,
                Priority::SHUTDOWN,
                expected_master_adver_interval,
            ))
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![Action::WaitForInput]);
        assert_eq!(
            *router.state(),
            State::Backup {
                master_down_timer: now
                    + (((256 - p.priority.as_u32()) * expected_master_adver_interval) / 256),
                master_adver_interval: expected_master_adver_interval,
            }
        );
    }

    #[test]
    fn backup_receive_greater_priority_advertisement() {
        let (mut router, p, now) = startup_with_priority(Priority::default());

        let expected_master_adver_interval = Interval::from_secs(5);
        let actions = router
            .handle_input(Input::Advertisement(
                now,
                Priority::new(101),
                expected_master_adver_interval,
            ))
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![Action::WaitForInput]);
        assert_eq!(
            *router.state(),
            State::Backup {
                master_down_timer: now
                    + (3 * expected_master_adver_interval)
                    + (((256 - p.priority.as_u32()) * expected_master_adver_interval) / 256),
                master_adver_interval: expected_master_adver_interval,
            }
        );
    }
}
