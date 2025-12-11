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

use protocol::ethernet::{EtherType, EthernetHeader};
use protocol::ipv4::{Ipv4Addr, Ipv4Header, Ipv4Protocol};
use protocol::mac::MacAddr;

use crate::{handlers::icmp, stack::NetworkStack};

pub fn handle(stack: &NetworkStack, eth_header: &EthernetHeader, payload: &[u8]) {
    if payload.len() <= 20 {
        eprintln!("error: IPv4 frame length less equal than 20");
        return;
    }

    let header_bytes: &[u8; 20] = match payload[0..20].try_into() {
        Ok(bytes) => bytes,
        Err(_) => {
            eprintln!("error: IPv4 frame length less equal than 20");
            return;
        }
    };

    let header = match Ipv4Header::parse(header_bytes) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Invalid IPv4 header: {}", e);
            return;
        }
    };

    if header.dst != stack.config().ip {
        // drop or resend(router)
        return;
    }

    if payload.len() < header.total_len as usize {
        // error length
        return;
    }

    match header.get_protocol() {
        Ipv4Protocol::ICMP => {
            icmp::handle(stack, eth_header.src, header.src, &payload[20..]);
        }
        Ipv4Protocol::TCP => {
            // drop
        }
        Ipv4Protocol::UDP => {
            // drop
        }
        Ipv4Protocol::Unknown => {
            eprintln!("Unknown IPv4 Protocol: {}", header.protocol)
        }
    }
}

pub fn send_packet(
    stack: &NetworkStack,
    dst_mac: MacAddr,
    dst_ip: Ipv4Addr,
    protocol: Ipv4Protocol,
    payload: &[u8],
) {
    let src_ip = stack.config().ip;
    // ID generation? Just use 0 or random for now.
    let id = 0;
    let protocol_u8 = match protocol {
        Ipv4Protocol::ICMP => 1,
        Ipv4Protocol::TCP => 6,
        Ipv4Protocol::UDP => 17,
        _ => 0,
    };

    let header = Ipv4Header::new(src_ip, dst_ip, protocol_u8, payload.len() as u16, id);
    let header_bytes = header.to_bytes();

    // Combine
    let mut frame_payload = Vec::new();
    frame_payload.extend_from_slice(&header_bytes);
    frame_payload.extend_from_slice(payload);

    // Ethernet Header
    let eth_header = EthernetHeader {
        dst: dst_mac,
        src: stack.config().mac,
        ethertype: EtherType::Ipv4,
    };

    let mut frame = Vec::new();
    frame.extend_from_slice(&eth_header.to_bytes());
    frame.extend_from_slice(&frame_payload);

    // Padding to minimum Ethernet frame size (60 bytes)
    if frame.len() < 60 {
        frame.resize(60, 0);
    }

    stack.send_frame(&frame);
}
