use crate::Parameters;
use pnet_base::MacAddr;
use std::net::Ipv4Addr;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SendPacket<'a> {
    Advertisement(&'a Parameters),
    ShutdownAdvertisement(&'a Parameters),
    GratuitousARP {
        sender_mac: MacAddr,
        sender_ip: Ipv4Addr,
    },
    ReplyARP {
        sender_mac: MacAddr,
        sender_ip: Ipv4Addr,
        target_mac: MacAddr,
        target_ip: Ipv4Addr,
    },
}

// VRRP advertisement
// {
//     // VRRP pakcet
//     let mut vrrp_buff: Vec<u8> = vec![0; 16 + (4 * vrouter.ip_addresses.len())];
//     let mut vrrp_packet = generator.gen_vrrp_header(&mut vrrp_buff, &vrouter);
//     vrrp_packet.set_checksum(checksum::one_complement_sum(vrrp_packet.packet(), Some(6)));
//
//     // IP packet
//     let ip_len = vrrp_packet.packet().len() + 20;
//     let mut ip_buff: Vec<u8> = vec![0; ip_len];
//     let mut ip_packet = generator.gen_vrrp_ip_header(&mut ip_buff);
//     ip_packet.set_payload(vrrp_packet.packet());
//
//     // Ethernet packet
//     let mut eth_buffer: Vec<u8> = vec![0; 14 + ip_packet.packet().len()];
//     let mut ether_packet = generator.gen_vrrp_eth_packet(&mut eth_buffer);
//     ether_packet.set_payload(ip_packet.packet());
//     sender.send_to(ether_packet.packet(), None);
// }
