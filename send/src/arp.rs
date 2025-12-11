// Copyright (C) 2025 rrrrrzy
// SPDX-License-Identifier: GPL-3.0-or-later
//
// --------------------------------------------------
// 致敬所有在深夜调试代码的灵魂。
// 即便 Bug 如山，我亦往矣。
// --------------------------------------------------
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

use protocol::arp::ArpOperation;
use protocol::ipv4::Ipv4Addr;
use protocol::mac::MacAddr;

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
