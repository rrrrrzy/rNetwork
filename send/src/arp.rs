use std::net::Ipv4Addr;

use crate::mac::MacAddr;

/// Supported ARP operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArpOperation {
    Request,
    Reply,
}

impl ArpOperation {
    fn opcode(self) -> u16 {
        match self {
            Self::Request => 1,
            Self::Reply => 2,
        }
    }
}

/// Build an ARP payload following RFC 826.
pub fn build_arp_payload(
    op: ArpOperation,
    sender_mac: MacAddr,
    sender_ip: Ipv4Addr,
    target_mac: MacAddr,
    target_ip: Ipv4Addr,
) -> Vec<u8> {
    let mut payload = Vec::with_capacity(28);
    payload.extend_from_slice(&1u16.to_be_bytes()); // Ethernet hardware type
    payload.extend_from_slice(&0x0800u16.to_be_bytes()); // Protocol type IPv4
    payload.push(6); // MAC length
    payload.push(4); // IPv4 length
    payload.extend_from_slice(&op.opcode().to_be_bytes());
    payload.extend_from_slice(sender_mac.as_bytes());
    payload.extend_from_slice(&sender_ip.octets());
    payload.extend_from_slice(target_mac.as_bytes());
    payload.extend_from_slice(&target_ip.octets());
    payload
}
