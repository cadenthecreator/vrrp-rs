mod actions;
mod input;
mod interval;
mod mode;
mod parameters;
mod priority;
mod received;
mod router;
mod send;
mod vrid;

pub use actions::{Action, RoutePacket};
pub use input::{Command, Input};
pub use interval::Interval;
pub use mode::{BackupMode, Mode};
pub use parameters::Parameters;
pub use priority::Priority;
pub use received::ReceivedPacket;
pub use router::{Router, State};
pub use send::SendPacket;
pub use vrid::VRID;

#[cfg(test)]
mod tests {
    use super::*;
    use pnet_base::MacAddr;
    use pretty_assertions::assert_eq;
    use std::net::Ipv4Addr;
    use std::num::NonZeroU8;
    use std::time::Instant;

    const TEST_PRIMARY_IP: Ipv4Addr = Ipv4Addr::new(42, 42, 42, 42);
    const TEST_SENDER_IP: Ipv4Addr = Ipv4Addr::new(24, 24, 24, 24);
    const TEST_SENDER_MAC: MacAddr = MacAddr(2, 5, 2, 5, 2, 5);

    fn default_mode() -> BackupMode {
        BackupMode::with_primary_ip(TEST_PRIMARY_IP)
    }

    fn startup_in(mode: impl Into<Mode>) -> (Router, Parameters, Instant) {
        let (mut router, parameters) = router_in(mode);
        let now = Instant::now();
        let _ = router.handle_input(now, Command::Startup.into());

        (router, parameters, now)
    }

    fn active_in(mode: impl Into<Mode>) -> (Router, Parameters, Instant) {
        let (mut router, parameters, now) = startup_in(mode);

        let now = now + Interval::from_secs(10);
        let _ = router.handle_input(now, Input::Timer);

        (router, parameters, now)
    }

    fn router_in(mode: impl Into<Mode>) -> (Router, Parameters) {
        let ip_1 = Ipv4Addr::new(1, 1, 1, 1);
        let ip_2 = Ipv4Addr::new(2, 2, 2, 2);
        let ip_addresses = vec![ip_1, ip_2];
        let advertisement_interval = Interval::from_secs(1);
        let parameters = Parameters {
            virtual_addresses: ip_addresses,
            advertisement_interval,
            mode: mode.into(),
            vrid: VRID::try_from(1).unwrap(),
        };

        let router = Router::new(parameters.clone());

        (router, parameters)
    }

    #[test]
    fn startup() {
        let (mut router, p) = router_in(default_mode());

        assert_eq!(
            *router.state(),
            State::Initialized,
            "all routers should begin in the initialized state"
        );

        let now = Instant::now();
        let actions = router
            .handle_input(now, Command::Startup.into())
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![]);
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
        let (mut router, p) = router_in(Mode::Owner);

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
            SendPacket::Advertisement(&p).into(),
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
        let (mut router, p, now) = startup_in(default_mode());

        let now = now + p.active_down_interval(p.advertisement_interval);
        let actions = router.handle_input(now, Input::Timer).collect::<Vec<_>>();

        assert_eq!(
            actions[0],
            Action::Activate,
            "it should Activate the virtual addresses on the router interface"
        );
        assert_eq!(
            actions[1],
            SendPacket::Advertisement(&p).into(),
            "it should Send an ADVERTISEMENT"
        );
        assert_eq!(*router.state(), State::Active { adver_timer: now + p.advertisement_interval }, "it should transition to the Active state and set the Adver_Timer to Advertisement_Interval");
    }

    #[test]
    fn backup_shutdown() {
        let (mut router, _, now) = startup_in(default_mode());

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
        let (mut router, p, now) = startup_in(Mode::Owner);

        let actions = router
            .handle_input(now, Command::Shutdown.into())
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![
                SendPacket::ShutdownAdvertisement(&p).into(),
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
    fn backup_receives_shutdown_advertisement() {
        let (mut router, _, now) = startup_in(default_mode());

        let expected_max_advertise_interval = Interval::from_secs(10);
        let actions = router
            .handle_input(
                now,
                ReceivedPacket::ShutdownAdvertisement {
                    max_advertise_interval: expected_max_advertise_interval,
                }
                .into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![]);
        assert_eq!(
            *router.state(),
            State::Backup {
                active_down_timer: now + 156 * expected_max_advertise_interval / 256,
                active_adver_interval: expected_max_advertise_interval,
            },
            "it should set the Active_Down_Timer to Skew_Time"
        );
    }

    #[test]
    fn backup_receives_greater_priority_advertisement() {
        let of = |priority| Priority::try_from(priority).unwrap();
        let (mut router, _, now) = startup_in(default_mode().with_priority(of(200)));

        let expected_max_advertise_interval = Interval::from_secs(5);
        let actions = router
            .handle_input(
                now,
                ReceivedPacket::Advertisement {
                    sender_ip: TEST_SENDER_IP,
                    priority: of(201).into(),
                    max_advertise_interval: expected_max_advertise_interval,
                }
                .into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![]);
        assert_eq!(
            *router.state(),
            State::Backup {
                active_down_timer: now
                    + 3 * expected_max_advertise_interval
                    + 56 * expected_max_advertise_interval / 256,
                active_adver_interval: expected_max_advertise_interval,
            },
            "it should set Active_Adver_Interval to Adver Interval contained in the ADVERTISEMENT, \
            recompute the Active_Down_Interval, and \
            reset the Active_Down_Timer to Active_Down_Interval"
        );
    }

    #[test]
    fn backup_receives_lower_priority_advertisement() {
        let (mut router, p, now) = startup_in(default_mode());

        let actions = router
            .handle_input(
                now,
                ReceivedPacket::Advertisement {
                    sender_ip: Ipv4Addr::new(0, 0, 0, 0),
                    priority: NonZeroU8::new(1).unwrap(),
                    max_advertise_interval: Interval::from_secs(5),
                }
                .into(),
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
    fn backup_receives_lower_priority_advertisement_non_preempt() {
        let (mut router, p, now) = startup_in(default_mode().with_preempt(false));

        let expected_max_advertise_interval = Interval::from_secs(5);
        let actions = router
            .handle_input(
                now,
                ReceivedPacket::Advertisement {
                    sender_ip: Ipv4Addr::new(0, 0, 0, 0),
                    priority: NonZeroU8::new(1).unwrap(),
                    max_advertise_interval: expected_max_advertise_interval,
                }
                .into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![]);
        assert_eq!(
            *router.state(),
            State::Backup {
                active_down_timer: now + p.active_down_interval(expected_max_advertise_interval),
                active_adver_interval: expected_max_advertise_interval,
            }
        );
    }

    #[test]
    fn active_receives_shutdown_advertisement() {
        let (mut router, p, now) = startup_in(Mode::Owner);

        let expected_max_advertise_interval = Interval::from_secs(10);
        let actions = router
            .handle_input(
                now,
                ReceivedPacket::ShutdownAdvertisement {
                    max_advertise_interval: expected_max_advertise_interval,
                }
                .into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(actions, vec![SendPacket::Advertisement(&p).into()]);
        assert_eq!(
            *router.state(),
            State::Active {
                adver_timer: now + p.advertisement_interval,
            }
        );
    }

    #[test]
    fn active_receives_greater_priority_advertisement() {
        let tests = [
            (200.try_into().unwrap(), Ipv4Addr::new(1, 1, 1, 1)),
            (Priority::default(), Ipv4Addr::new(9, 9, 9, 9)),
        ];
        for (sender_priority, sender_ip) in tests {
            let (mut router, p, now) = active_in(default_mode());

            let expected_max_advertise_interval = Interval::from_secs(10);
            let actions = router
                .handle_input(
                    now,
                    ReceivedPacket::Advertisement {
                        sender_ip,
                        priority: sender_priority.into(),
                        max_advertise_interval: expected_max_advertise_interval,
                    }
                    .into(),
                )
                .collect::<Vec<_>>();

            assert_eq!(
                actions,
                vec![Action::Deactivate],
                "({sender_priority:?}, {sender_ip:?})"
            );
            assert_eq!(
                *router.state(),
                State::Backup {
                    active_adver_interval: expected_max_advertise_interval,
                    active_down_timer: now + p.active_down_interval(expected_max_advertise_interval),
                },
                "it should Set Active_Adver_Interval to Max Advertise Interval contained in the ADVERTISEMENT, \
                 Recompute the Active_Down_Interval, \
                 Set Active_Down_Timer to Active_Down_Interval and \
                 Transition to the Backup state"
            );
        }
    }

    #[test]
    fn active_receives_lower_priority_advertisement() {
        let tests = [Priority::default(), 1.try_into().unwrap()];
        for sender_priority in tests {
            let (mut router, p, now) = active_in(default_mode());

            let initial_state = router.state().clone();

            let expected_max_advertise_interval = Interval::from_secs(10);
            let actions = router
                .handle_input(
                    now,
                    ReceivedPacket::Advertisement {
                        sender_ip: Ipv4Addr::new(1, 1, 1, 1),
                        priority: sender_priority.into(),
                        max_advertise_interval: expected_max_advertise_interval,
                    }
                    .into(),
                )
                .collect::<Vec<_>>();

            assert_eq!(actions, vec![SendPacket::Advertisement(&p).into()],
                "it should Send an ADVERTISEMENT immediately to assert the Active state to the sending VRRP Router \
                and to update any learning bridges with the correct Active VRRP Router path."
            );

            assert_eq!(*router.state(), initial_state, "it should NOT change state");
        }
    }

    #[test]
    fn active_adver_timer_fires() {
        let (mut router, p, now) = startup_in(Mode::Owner);

        let now = now + Interval::from_centis(1);
        let actions = router.handle_input(now, Input::Timer).collect::<Vec<_>>();
        assert_eq!(actions, vec![]);

        let now = now + p.advertisement_interval;
        let actions = router.handle_input(now, Input::Timer).collect::<Vec<_>>();

        assert_eq!(actions, vec![SendPacket::Advertisement(&p).into()]);
        assert_eq!(
            *router.state(),
            State::Active {
                adver_timer: now + p.advertisement_interval,
            },
            "it should Reset the Adver_Timer to Advertisement_Interval"
        );
    }

    #[test]
    fn active_receives_arp_request() {
        let (mut router, p, now) = startup_in(Mode::Owner);

        let actions = router
            .handle_input(
                now,
                ReceivedPacket::RequestARP {
                    sender_mac: TEST_SENDER_MAC,
                    sender_ip: TEST_SENDER_IP,
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
                target_mac: TEST_SENDER_MAC,
                target_ip: TEST_SENDER_IP,
            }
            .into()]
        );
    }

    #[test]
    fn active_receives_ip_packet_forwarded() {
        let (mut router, p, now) = startup_in(Mode::Owner);

        let target_ip = Ipv4Addr::new(5, 2, 5, 2);
        let actions = router
            .handle_input(
                now,
                ReceivedPacket::IP {
                    target_mac: p.mac_address(),
                    target_ip,
                }
                .into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![RoutePacket::Forward.into()], "MUST forward packets with a destination link-layer MAC address equal to the virtual router MAC address.");
    }

    #[test]
    fn active_receives_ip_packet_accepted() {
        let (mut router, p, now) = startup_in(Mode::Owner);

        let target_ip = p.ipv4(0);
        let actions = router
            .handle_input(
                now,
                ReceivedPacket::IP {
                    target_mac: p.mac_address(),
                    target_ip,
                }
                .into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![RoutePacket::Accept.into()], "it MUST accept packets addressed to the IPvX address(es) associated with the virtual router if it is the IPvX address owner.");
    }

    #[test]
    fn active_accept_mode_receives_ip_packet() {
        let (mut router, p, now) = active_in(default_mode().with_accept(true));

        let target_ip = p.ipv4(0);
        let actions = router
            .handle_input(
                now,
                ReceivedPacket::IP {
                    target_mac: p.mac_address(),
                    target_ip,
                }
                .into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![RoutePacket::Accept.into()], "it MUST accept packets addressed to the IPvX address(es) associated with the virtual router if Accept_Mode is True.");
    }

    #[test]
    fn active_receives_ip_packet_discarded() {
        let (mut router, _, now) = startup_in(Mode::Owner);

        let actions = router
            .handle_input(
                now,
                ReceivedPacket::IP {
                    target_mac: TEST_SENDER_MAC,
                    target_ip: TEST_SENDER_IP,
                }
                .into(),
            )
            .collect::<Vec<_>>();

        assert_eq!(
            actions,
            vec![],"MUST not forward or accept packets with a destination link-layer MAC address not equal to the virtual router MAC address.");
    }
}
