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

use crate::handlers;
use crate::stack::NetworkStack;
use protocol::{
    arp::{ArpOperation, ArpPacket},
    ethernet::{EtherType, EthernetHeader},
    ipv4::Ipv4Addr,
    mac::MacAddr,
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

    // 使用作用域限制锁的生命周期
    {
        // 这里使用 unwrap，是因为如果锁被 poison，说明程序已经处于不一致状态，应该 panic 而不是继续执行
        let mut arp_table = stack.arp_table().lock().unwrap();
        arp_table.insert(packet.sender_ip, packet.sender_mac);
        println!(
            "学习到 ARP 映射: {} -> {}",
            packet.sender_ip, packet.sender_mac
        );
    }

    // 检查是否有等待这个 IP 的包
    {
        let mut pending = stack.pending_packets().lock().unwrap();
        if let Some(packets) = pending.remove(&packet.sender_ip) {
            println!(
                "发现 {} 个等待 {} 的数据包，正在发送...",
                packets.len(),
                packet.sender_ip
            );
            for pkt in packets {
                handlers::ipv4::send_packet_with_mac(
                    stack,
                    packet.sender_mac, // 现在我们知道 MAC 了
                    pkt.dst_ip,
                    pkt.protocol,
                    &pkt.payload,
                );
            }
        }
    }

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

pub fn send_request(stack: &NetworkStack, target_ip: Ipv4Addr) {
    let request_packet = ArpPacket {
        hardware_type: 1,
        protocol_type: 0x0800,
        hardware_len: 6,
        protocol_len: 4,
        opcode: 1, // Request
        sender_mac: stack.config().mac,
        sender_ip: stack.config().ip,
        target_mac: MacAddr::zero(), // 未知，填 00:00:00:00:00:00
        target_ip,
    };

    let payload = request_packet.to_bytes();

    // 构造以太网帧（广播）
    let eth_header = protocol::ethernet::EthernetHeader {
        dst: MacAddr::broadcast(), // FF:FF:FF:FF:FF:FF
        src: stack.config().mac,
        ethertype: protocol::ethernet::EtherType::Arp,
    };

    let mut frame = Vec::new();
    frame.extend_from_slice(&eth_header.to_bytes());
    frame.extend_from_slice(&payload);

    if frame.len() < 60 {
        frame.resize(60, 0);
    }

    stack.send_frame(&frame);
    println!("已发送 ARP 请求: 谁是 {}?", target_ip);
}
