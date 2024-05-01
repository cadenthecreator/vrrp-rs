mod interval;
mod parameters;
mod priority;
mod router;

use pnet_base::MacAddr;
use std::net::Ipv4Addr;

pub use interval::Interval;
pub use parameters::Parameters;
pub use priority::Priority;
pub use router::{Action, Input, Router, State};

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    use crate::router::{ArpReply, IpPacket};
    use pretty_assertions::assert_eq;

    fn startup_with_priority(priority: Priority) -> (Router, Parameters, Instant) {
        let (mut router, parameters) = router_with_priority(priority);
        let now = Instant::now();
        let _ = router.handle_input(Input::Startup(now));
        (router, parameters, now)
    }

    fn router_with_priority(priority: Priority) -> (Router, Parameters) {
        let ip_1 = Ipv4Addr::new(1, 1, 1, 1);
        let ip_2 = Ipv4Addr::new(2, 2, 2, 2);
        let ip_addresses = vec![ip_1, ip_2];
        let advertisement_interval = Interval::from_secs(1);
        let mac_address = MacAddr::new(1, 1, 1, 1, 1, 1);

        let parameters = Parameters {
            mac_address,
            ip_addresses,
            advertisement_interval,
            priority,
        };

        let router = Router::new(parameters.clone());

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
            Action::SendAdvertisement(Priority::OWNER, p.advertisement_interval),
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

        let now = now + p.master_down_interval(p.advertisement_interval);
        let actions = router.handle_input(Input::Timer(now)).collect::<Vec<_>>();
        assert_eq!(
            actions[0],
            Action::SendAdvertisement(Priority::new(100), p.advertisement_interval),
            "it should Send an ADVERTISEMENT"
        );
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
        let (mut router, p, _) = startup_with_priority(Priority::OWNER);

        let actions = router.handle_input(Input::Shutdown).collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![Action::SendAdvertisement(
                Priority::SHUTDOWN,
                p.advertisement_interval
            )]
        );
        assert_eq!(
            *router.state(),
            State::Initialized,
            "all routers should end in the initialized state"
        );
    }

    #[test]
    fn backup_receive_shutdown_advertisement() {
        let (mut router, _, now) = startup_with_priority(Priority::default());

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
                master_down_timer: now + 156 * expected_master_adver_interval / 256,
                master_adver_interval: expected_master_adver_interval,
            },
            "it should set the Master_Down_Timer to Skew_Time"
        );
    }

    #[test]
    fn backup_receive_greater_priority_advertisement() {
        let (mut router, _, now) = startup_with_priority(Priority::new(200));

        let expected_master_adver_interval = Interval::from_secs(5);
        let actions = router
            .handle_input(Input::Advertisement(
                now,
                Priority::new(201),
                expected_master_adver_interval,
            ))
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![Action::WaitForInput]);
        assert_eq!(
            *router.state(),
            State::Backup {
                master_down_timer: now
                    + 3 * expected_master_adver_interval
                    + 56 * expected_master_adver_interval / 256,
                master_adver_interval: expected_master_adver_interval,
            },
            "it should set Master_Adver_Interval to Adver Interval contained in the ADVERTISEMENT, \
            recompute the Master_Down_Interval, and \
            reset the Master_Down_Timer to Master_Down_Interval"
        );
    }

    #[test]
    fn backup_receive_lower_priority_advertisement() {
        let (mut router, p, now) = startup_with_priority(Priority::default());

        let actions = router
            .handle_input(Input::Advertisement(
                now,
                Priority::new(1),
                Interval::from_secs(5),
            ))
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![Action::WaitForInput]);
        assert_eq!(
            *router.state(),
            State::Backup {
                master_down_timer: now
                    + 3 * p.advertisement_interval
                    + 156 * p.advertisement_interval / 256,
                master_adver_interval: p.advertisement_interval,
            },
            "it should set Master_Adver_Interval to Adver Interval contained in the ADVERTISEMENT, \
            recompute the Master_Down_Interval, and \
            reset the Master_Down_Timer to Master_Down_Interval"
        );
    }

    #[test]
    fn master_receive_shutdown_advertisement() {
        let (mut router, p, now) = startup_with_priority(Priority::OWNER);

        let expected_master_adver_interval = Interval::from_secs(10);
        let actions = router
            .handle_input(Input::Advertisement(
                now,
                Priority::SHUTDOWN,
                expected_master_adver_interval,
            ))
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![Action::SendAdvertisement(
                p.priority,
                p.advertisement_interval
            )]
        );
        assert_eq!(
            *router.state(),
            State::Master {
                adver_timer: now + p.advertisement_interval,
            },
            "it should set the Master_Down_Timer to Skew_Time"
        );
    }

    #[test]
    fn master_greater_priority_advertisement() {
        let (mut router, p, now) = startup_with_priority(Priority::default());

        let now = now + p.master_down_interval(p.advertisement_interval);
        let _ = router.handle_input(Input::Timer(now)).collect::<Vec<_>>();
        let expected_master_adver_interval = Interval::from_secs(10);
        let actions = router
            .handle_input(Input::Advertisement(
                now,
                Priority::OWNER,
                expected_master_adver_interval,
            ))
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![Action::WaitForInput]);
        assert_eq!(
            *router.state(),
            State::Backup {
                master_adver_interval: expected_master_adver_interval,
                master_down_timer: now + p.master_down_interval(expected_master_adver_interval),
            },
            "it should Set Master_Adver_Interval to Adver Interval contained in the ADVERTISEMENT and \
             Recompute the Master_Down_Interval and \
             Set Master_Down_Timer to Master_Down_Interval and \
             Transition to the Backup state"
        );
    }
    #[test]
    fn master_adver_timer_fires() {
        let (mut router, p, now) = startup_with_priority(Priority::OWNER);

        let now = now + p.advertisement_interval;
        let actions = router.handle_input(Input::Timer(now)).collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![Action::SendAdvertisement(
                p.priority,
                p.advertisement_interval
            )]
        );
        assert_eq!(
            *router.state(),
            State::Master {
                adver_timer: now + p.advertisement_interval,
            },
            "it should Reset the Adver_Timer to Advertisement_Interval"
        );
    }

    #[test]
    fn master_arp_request() {
        let (mut router, p, _) = startup_with_priority(Priority::OWNER);

        let actions = router
            .handle_input(Input::ARP {
                sender_mac: MacAddr::new(2, 5, 2, 5, 2, 5),
                sender_ip: Ipv4Addr::new(2, 5, 2, 5),
                target_ip: p.ipv4(0),
            })
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![Action::SendARP(ArpReply {
                sender_mac: p.mac_address,
                sender_ip: p.ipv4(0),
                target_mac: MacAddr::new(2, 5, 2, 5, 2, 5),
                target_ip: Ipv4Addr::new(2, 5, 2, 5),
            })]
        );
    }

    #[test]
    fn master_receive_ip_packet_forwarded() {
        let (mut router, p, _) = startup_with_priority(Priority::OWNER);
        let data = [8u8, 8u8, 8u8, 8u8];

        let packet = IpPacket {
            sender_ip: Ipv4Addr::new(2, 5, 2, 5),
            target_ip: Ipv4Addr::new(5, 2, 5, 2),
            data: &data,
        };
        let actions = router
            .handle_input(Input::IpPacket(p.mac_address, packet))
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![Action::ForwardPacket(packet)],"MUST forward packets with a destination link-layer MAC address equal to the virtual router MAC address.");

        let packet = IpPacket {
            sender_ip: Ipv4Addr::new(2, 5, 2, 5),
            target_ip: p.ipv4(0),
            data: &data,
        };
        let actions = router
            .handle_input(Input::IpPacket(p.mac_address, packet))
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![Action::AcceptPacket(packet)],"MUST forward packets with a destination link-layer MAC address equal to the virtual router MAC address.");
    }

    #[test]
    fn master_receive_ip_packet_discarded() {
        let (mut router, p, _) = startup_with_priority(Priority::OWNER);
        let data = [8u8, 8u8, 8u8, 8u8];

        let packet = IpPacket {
            sender_ip: Ipv4Addr::new(2, 5, 2, 5),
            target_ip: p.ipv4(0),
            data: &data,
        };
        let actions = router
            .handle_input(Input::IpPacket(MacAddr::new(2, 5, 2, 5, 2, 5), packet))
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![Action::WaitForInput],"MUST not forward packets with a destination link-layer MAC address not equal to the virtual router MAC address.");

        let packet = IpPacket {
            sender_ip: Ipv4Addr::new(2, 5, 2, 5),
            target_ip: Ipv4Addr::new(5, 2, 5, 2),
            data: &data,
        };
        let actions = router
            .handle_input(Input::IpPacket(p.mac_address, packet))
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![Action::ForwardPacket(packet)],"MUST accept packets addressed to the IPvX address(es) associated with the virtual router if it is the IPvX address owner.  Otherwise, MUST NOT accept these packets.");
    }
}
