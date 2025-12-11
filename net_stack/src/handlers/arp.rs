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

use crate::stack::NetworkStack;
use protocol::{
    arp::{ArpOperation, ArpPacket},
    ethernet::{EtherType, EthernetHeader},
};

pub fn handle(stack: &NetworkStack, payload: &[u8]) {
    // 1. parse
    let packet = match ArpPacket::parse(payload) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Invalid ARP packet: {}", e);
            return;
        }
    };

    match ArpOperation::parse(packet.opcode) {
        ArpOperation::Request => {
            if packet.target_ip == stack.config().ip {
                println!(
                    "收到 ARP 请求: 谁是 {}? (来自 {})",
                    packet.target_ip, packet.sender_ip
                );

                send_reply(stack, &packet);
            }
        }
        ArpOperation::Reply => {
            println!(
                "收到 ARP 响应: {} 在 {}",
                packet.sender_ip, packet.sender_mac
            );
            // TODO: update ARP table
        }
        _ => { // do nothing
        }
    }
}

fn send_reply(stack: &NetworkStack, request: &ArpPacket) {
    let reply_packet = ArpPacket::new(
        ArpOperation::Reply,
        stack.config().mac, // Sender MAC (我)
        stack.config().ip,  // Sender IP (我)
        request.sender_mac, // Target MAC (对方)
        request.sender_ip,  // Target IP (对方)
    );

    let eth_header = EthernetHeader::new(stack.config().mac, request.sender_mac, EtherType::Arp);

    let mut frame = eth_header.to_bytes().to_vec();
    frame.extend_from_slice(&reply_packet.to_bytes());

    // Padding to minimum Ethernet frame size (60 bytes + 4 CRC = 64 bytes)
    // Header (14) + ARP (28) = 42 bytes. Need 18 bytes padding.
    if frame.len() < 60 {
        frame.resize(60, 0);
    }

    stack.send_frame(&frame);
}
