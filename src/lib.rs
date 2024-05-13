mod actions;
mod input;
mod interval;
mod parameters;
mod priority;
mod router;
mod vrid;

use pnet_base::MacAddr;
use std::net::Ipv4Addr;

pub use actions::{Action, RoutePacket, SendPacket};
pub use input::{Command, Input, ReceivedPacket};
pub use interval::Interval;
pub use parameters::Parameters;
pub use priority::Priority;
pub use router::{Router, State};
pub use vrid::VRID;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{RoutePacket, SendPacket};
    use crate::input::ReceivedPacket;
    use pretty_assertions::assert_eq;
    use std::time::Instant;

    fn startup_with_priority(priority: Priority) -> (Router, Parameters, Instant) {
        let (mut router, parameters) = router_with(priority, true, false);
        let now = Instant::now();
        let _ = router.handle_input(now, Command::Startup.into());
        (router, parameters, now)
    }

    fn startup_with_accept_mode(accept_mode: bool) -> (Router, Parameters, Instant) {
        let (mut router, parameters) = router_with(Priority::default(), true, accept_mode);
        let now = Instant::now();
        let _ = router.handle_input(now, Command::Startup.into());
        let now = now + parameters.active_down_interval(parameters.advertisement_interval);
        let _ = router.handle_input(now, Input::Timer).collect::<Vec<_>>();

        (router, parameters, now)
    }

    fn startup_with_preempt_mode(preempt_mode: bool) -> (Router, Parameters, Instant) {
        let (mut router, parameters) = router_with(Priority::default(), preempt_mode, false);
        let now = Instant::now();
        let _ = router.handle_input(now, Command::Startup.into());
        (router, parameters, now)
    }

    fn router_with(
        priority: Priority,
        preempt_mode: bool,
        accept_mode: bool,
    ) -> (Router, Parameters) {
        let ip_1 = Ipv4Addr::new(1, 1, 1, 1);
        let ip_2 = Ipv4Addr::new(2, 2, 2, 2);
        let ip_addresses = vec![ip_1, ip_2];
        let advertisement_interval = Interval::from_secs(1);

        let parameters = Parameters {
            ipv4_addresses: ip_addresses,
            advertisement_interval,
            preempt_mode,
            accept_mode,
            priority,
            vrid: VRID::try_from(1).unwrap(),
        };

        let router = Router::new(parameters.clone());

        (router, parameters)
    }

    #[test]
    fn startup() {
        let (mut router, p) = router_with(Priority::default(), true, false);

        assert_eq!(
            *router.state(),
            State::Initialized,
            "all routers should begin in the initialized state"
        );

        let now = Instant::now();
        assert_eq!(
            router
                .handle_input(now, Command::Startup.into())
                .collect::<Vec<_>>(),
            vec![]
        );
        assert_eq!(
            *router.state(),
            State::Backup {
                active_down_timer: now
                    + 3 * p.advertisement_interval
                    + ((256 - 100) * p.advertisement_interval / 256),
                active_adver_interval: p.advertisement_interval,
            },
            "after startup, an un-owned router should transition to the Backup state"
        );
    }

    #[test]
    fn startup_address_owner() {
        let (mut router, p) = router_with(Priority::OWNER, true, false);

        assert_eq!(
            *router.state(),
            State::Initialized,
            "all routers should begin in the initialized state"
        );

        // On Startup
        // If the router owns the IP address(es) associated with the virtual router
        let now = Instant::now();
        let actions = router
            .handle_input(now, Command::Startup.into())
            .collect::<Vec<_>>();
        assert_eq!(
            actions[0],
            Action::Activate,
            "it should Activate the virtual address on the router interface"
        );
        assert_eq!(
            actions[1],
            SendPacket::Advertisement(Priority::OWNER, p.advertisement_interval).into(),
            "it should Send an ADVERTISEMENT"
        );
        assert_eq!(vec![actions[2], actions[3]], vec![SendPacket::GratuitousARP { sender_mac: p.mac_address(), sender_ip: p.ipv4(0) }.into(), SendPacket::GratuitousARP { sender_mac: p.mac_address(), sender_ip: p.ipv4(1) }.into()], "for each IP address associated with the virtual router, it should broadcast a gratuitous ARP request containing the virtual router MAC address");
        assert_eq!(
            *router.state(),
            State::Active {
                adver_timer: now + p.advertisement_interval
            },
            "after startup, an owned router should transition to the Active state"
        );
    }

    #[test]
    fn backup_active_down_timer_fires() {
        let (mut router, p, now) = startup_with_priority(Priority::default());

        let now = now + p.active_down_interval(p.advertisement_interval);
        let actions = router.handle_input(now, Input::Timer).collect::<Vec<_>>();
        assert_eq!(
            actions[0],
            Action::Activate,
            "it should Activate the virtual addresses on the router interface"
        );
        assert_eq!(
            actions[1],
            SendPacket::Advertisement(Priority::new(100), p.advertisement_interval).into(),
            "it should Send an ADVERTISEMENT"
        );
        assert_eq!(*router.state(), State::Active { adver_timer: now + p.advertisement_interval }, "it should transition to the Active state and set the Adver_Timer to Advertisement_Interval");
    }

    #[test]
    fn backup_shutdown() {
        let (mut router, _, now) = startup_with_priority(Priority::default());

        let actions = router
            .handle_input(now, Command::Shutdown.into())
            .collect::<Vec<_>>();

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
    fn active_shutdown() {
        let (mut router, p, now) = startup_with_priority(Priority::OWNER);

        let actions = router
            .handle_input(now, Command::Shutdown.into())
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![
                SendPacket::Advertisement(Priority::SHUTDOWN, p.advertisement_interval).into(),
                Action::Deactivate,
            ]
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

        let expected_active_adver_interval = Interval::from_secs(10);
        let actions = router
            .handle_input(
                now,
                ReceivedPacket::Advertisement {
                    priority: Priority::SHUTDOWN,
                    active_adver_interval: expected_active_adver_interval
                }
                    .into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![]);
        assert_eq!(
            *router.state(),
            State::Backup {
                active_down_timer: now + 156 * expected_active_adver_interval / 256,
                active_adver_interval: expected_active_adver_interval,
            },
            "it should set the Active_Down_Timer to Skew_Time"
        );
    }

    #[test]
    fn backup_receive_greater_priority_advertisement() {
        let (mut router, _, now) = startup_with_priority(Priority::new(200));

        let expected_active_adver_interval = Interval::from_secs(5);
        let actions = router
            .handle_input(
                now,
                ReceivedPacket::Advertisement {
                    priority: Priority::new(201),
                    active_adver_interval: expected_active_adver_interval
                }
                    .into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![]);
        assert_eq!(
            *router.state(),
            State::Backup {
                active_down_timer: now
                    + 3 * expected_active_adver_interval
                    + 56 * expected_active_adver_interval / 256,
                active_adver_interval: expected_active_adver_interval,
            },
            "it should set Active_Adver_Interval to Adver Interval contained in the ADVERTISEMENT, \
            recompute the Active_Down_Interval, and \
            reset the Active_Down_Timer to Active_Down_Interval"
        );
    }

    #[test]
    fn backup_receive_lower_priority_advertisement() {
        let (mut router, p, now) = startup_with_priority(Priority::default());

        let actions = router
            .handle_input(
                now,
                ReceivedPacket::Advertisement {
                    priority: Priority::new(1),
                    active_adver_interval: Interval::from_secs(5)
                }.into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![]);
        assert_eq!(
            *router.state(),
            State::Backup {
                active_down_timer: now
                    + 3 * p.advertisement_interval
                    + 156 * p.advertisement_interval / 256,
                active_adver_interval: p.advertisement_interval,
            }
        );
    }

    #[test]
    fn backup_receive_lower_priority_advertisement_non_preempt() {
        let (mut router, p, now) = startup_with_preempt_mode(false);

        let expected_active_adver_interval = Interval::from_secs(5);
        let actions = router
            .handle_input(
                now,
                ReceivedPacket::Advertisement {
                    priority: Priority::new(1),
                    active_adver_interval: expected_active_adver_interval
                }
                    .into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![]);
        assert_eq!(
            *router.state(),
            State::Backup {
                active_down_timer: now + p.active_down_interval(expected_active_adver_interval),
                active_adver_interval: expected_active_adver_interval,
            }
        );
    }

    #[test]
    fn active_receive_shutdown_advertisement() {
        let (mut router, p, now) = startup_with_priority(Priority::OWNER);

        let expected_active_adver_interval = Interval::from_secs(10);
        let actions = router
            .handle_input(
                now,
                ReceivedPacket::Advertisement {
                    priority: Priority::SHUTDOWN,
                    active_adver_interval: expected_active_adver_interval
                }
                    .into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![SendPacket::Advertisement(p.priority, p.advertisement_interval).into()]
        );
        assert_eq!(
            *router.state(),
            State::Active {
                adver_timer: now + p.advertisement_interval,
            }
        );
    }

    #[test]
    fn active_greater_priority_advertisement() {
        let (mut router, p, now) = startup_with_accept_mode(false);

        let expected_active_adver_interval = Interval::from_secs(10);
        let actions = router
            .handle_input(
                now,
                ReceivedPacket::Advertisement {
                    priority: Priority::OWNER,
                    active_adver_interval: expected_active_adver_interval
                }
                    .into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![Action::Deactivate]);
        assert_eq!(
            *router.state(),
            State::Backup {
                active_adver_interval: expected_active_adver_interval,
                active_down_timer: now + p.active_down_interval(expected_active_adver_interval),
            },
            "it should Set Active_Adver_Interval to Adver Interval contained in the ADVERTISEMENT, \
             Recompute the Active_Down_Interval, \
             Set Active_Down_Timer to Active_Down_Interval and \
             Transition to the Backup state"
        );
    }

    #[test]
    fn active_adver_timer_fires() {
        let (mut router, p, now) = startup_with_priority(Priority::OWNER);

        let now = now + Interval::from_centis(1);
        let actions = router.handle_input(now, Input::Timer).collect::<Vec<_>>();
        assert_eq!(actions, vec![]);

        let now = now + p.advertisement_interval;
        let actions = router.handle_input(now, Input::Timer).collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![SendPacket::Advertisement(p.priority, p.advertisement_interval).into()]
        );
        assert_eq!(
            *router.state(),
            State::Active {
                adver_timer: now + p.advertisement_interval,
            },
            "it should Reset the Adver_Timer to Advertisement_Interval"
        );
    }

    #[test]
    fn active_arp_request() {
        let (mut router, p, now) = startup_with_priority(Priority::OWNER);

        let actions = router
            .handle_input(
                now,
                ReceivedPacket::RequestARP {
                    sender_mac: MacAddr::new(2, 5, 2, 5, 2, 5),
                    sender_ip: Ipv4Addr::new(2, 5, 2, 5),
                    target_ip: p.ipv4(0),
                }
                .into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![SendPacket::ReplyARP {
                sender_mac: p.mac_address(),
                sender_ip: p.ipv4(0),
                target_mac: MacAddr::new(2, 5, 2, 5, 2, 5),
                target_ip: Ipv4Addr::new(2, 5, 2, 5),
            }
            .into()]
        );
    }

    #[test]
    fn active_receive_ip_packet_forwarded() {
        let (mut router, p, now) = startup_with_priority(Priority::OWNER);

        let target_ip = Ipv4Addr::new(5, 2, 5, 2);
        let actions = router
            .handle_input(
                now,
                ReceivedPacket::IP { target_mac: p.mac_address(), target_ip }.into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![RoutePacket::Forward.into()], "MUST forward packets with a destination link-layer MAC address equal to the virtual router MAC address.");
    }

    #[test]
    fn active_receive_ip_packet_accepted() {
        let (mut router, p, now) = startup_with_priority(Priority::OWNER);

        let target_ip = p.ipv4(0);
        let actions = router
            .handle_input(
                now,
                ReceivedPacket::IP { target_mac: p.mac_address(), target_ip }.into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![RoutePacket::Accept.into()], "it MUST accept packets addressed to the IPvX address(es) associated with the virtual router if it is the IPvX address owner.");
    }

    #[test]
    fn active_accept_mode_receive_ip_packet() {
        let (mut router, p, now) = startup_with_accept_mode(true);

        let target_ip = p.ipv4(0);
        let actions = router
            .handle_input(
                now,
                ReceivedPacket::IP { target_mac: p.mac_address(), target_ip }.into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![RoutePacket::Accept.into()], "it MUST accept packets addressed to the IPvX address(es) associated with the virtual router if Accept_Mode is True.");
    }

    #[test]
    fn active_receive_ip_packet_discarded() {
        let (mut router, _, now) = startup_with_priority(Priority::OWNER);

        let target_ip = Ipv4Addr::new(5, 2, 5, 2);
        let actions = router
            .handle_input(
                now,
                ReceivedPacket::IP { target_mac: MacAddr::new(2, 5, 2, 5, 2, 5), target_ip }.into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![],"MUST not forward or accept packets with a destination link-layer MAC address not equal to the virtual router MAC address.");
    }
}
