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

use protocol::icmp::{ICMP, IcmpType};
use protocol::ipv4::{Ipv4Addr, Ipv4Protocol};

use crate::handlers::ipv4;
use crate::stack::NetworkStack;

pub fn handle(stack: &NetworkStack, src_ip: Ipv4Addr, payload: &[u8]) {
    let packet = match ICMP::parse(payload) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Invalid ICMP: {:?}", e);
            return;
        }
    };

    match IcmpType::parse(packet.header.type_) {
        IcmpType::Request => {
            println!("Received ICMP Request from {}", src_ip);
            println!("{}", packet);
            send_reply(stack, src_ip, &packet);
        }
        IcmpType::Reply => {
            println!("Received ICMP Reply from {}", src_ip);
            println!("{}", packet);
        }
        IcmpType::Unknown => {
            eprintln!("error get unknown icmp type: {}.", packet.header.type_);
        }
    }
}

fn send_reply(stack: &NetworkStack, dst_ip: Ipv4Addr, request: &ICMP) {
    let reply_packet = ICMP::new(
        IcmpType::Reply,
        request.header.code,
        request.header.id,
        request.header.seq,
        request.data.time,
        &request.data.payload,
    );

    let payload = reply_packet.to_bytes();
    ipv4::send_packet(stack, dst_ip, Ipv4Protocol::ICMP, &payload);
}

pub fn send_icmp_request(stack: &NetworkStack, dst_ip: Ipv4Addr, seq: u16) {
    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u32;

    let payload_data = vec![0u8; 32]; // 32 bytes payload

    let request_packet = ICMP::new(
        IcmpType::Request,
        0,    // code
        1234, // id
        seq,
        time,
        &payload_data,
    );

    let payload = request_packet.to_bytes();
    ipv4::send_packet(stack, dst_ip, Ipv4Protocol::ICMP, &payload);
}
