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

use protocol::{ipv4::Ipv4Addr, udp::UdpPacket};

use crate::{
    stack::NetworkStack,
    transport::{Socket, SocketType},
};

// receive
pub fn handle(stack: &NetworkStack, src_ip: Ipv4Addr, dst_ip: Ipv4Addr, payload: &[u8]) {
    let packet = match UdpPacket::parse(payload) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Invalid UDP packet: {:?}", e);
            return;
        }
    };

    // 防御性检查：确保目的 IP 是我们关心的
    if !(dst_ip == stack.config().ip || dst_ip.is_broadcast() || dst_ip.is_multicast()) {
        return;
    }

    // validate
    match packet.validate(src_ip, dst_ip) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Invalid UDP Packet: {:?}", e);
            return;
        }
    }

    // 先查找 socket，获取必要的引用，然后立即释放锁
    let should_enqueue = {
        if let Ok(mut socket_set) = stack.sockets.lock() {
            if let Some(socket) = socket_set.lookup(
                &SocketType::Udp,
                src_ip,
                packet.header.src_port,
                dst_ip,
                packet.header.dst_port,
            ) {
                match socket {
                    Socket::Udp(udp_socket) => {
                        // 直接在这里入队（因为已经拿到了锁）
                        udp_socket.rx_enqueue(src_ip, packet.header.src_port, &packet.payload);
                        true
                    }
                    _ => false,
                }
            } else {
                false
            }
        } else {
            false
        }
    }; // socket_set 锁在这里释放

    if !should_enqueue {
        // drop now
        // future: send ICMP Port Unreachable
    }
}
