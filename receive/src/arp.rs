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

use std::collections::HashMap;

use anyhow::Result;

use protocol::arp::ArpPacket;
use protocol::error::ArpParseError;
use protocol::ipv4::Ipv4Addr;
use protocol::mac::MacAddr;

pub struct ArpProcessor {
    allowed_targets: Vec<Ipv4Addr>,
    cache: HashMap<Ipv4Addr, MacAddr>,
}

impl ArpProcessor {
    pub fn new(allowed_targets: Vec<Ipv4Addr>) -> Self {
        Self {
            allowed_targets,
            cache: HashMap::new(),
        }
    }

    pub fn process(&mut self, payload: &[u8]) -> Result<(), ArpParseError> {
        let packet = ArpPacket::parse(payload)?;

        if !packet.is_ethernet_ipv4() {
            println!(
                "忽略非 Ethernet/IPv4 ARP 报文 (hw=0x{:#06x}, proto=0x{:#06x})",
                packet.hardware_type, packet.protocol_type
            );
            return Ok(());
        }

        if !self.allowed_targets.is_empty() && !self.allowed_targets.contains(&packet.target_ip) {
            println!("ARP 目标 IP {} 不在白名单，忽略。", packet.target_ip);
            return Ok(());
        }

        println!("--------------ARP Protocol-----------------");
        println!(
            "操作类型: {} (0x{:04x})",
            packet.opcode_label(),
            packet.opcode
        );
        println!("硬件类型: 0x{:04x}", packet.hardware_type);
        println!("协议类型: 0x{:04x}", packet.protocol_type);
        println!(
            "发送端 MAC/IP: {} / {}",
            packet.sender_mac, packet.sender_ip
        );
        println!("目标 MAC/IP: {} / {}", packet.target_mac, packet.target_ip);

        if self
            .cache
            .insert(packet.sender_ip, packet.sender_mac)
            .map_or(true, |old| old != packet.sender_mac)
        {
            println!(
                "ARP 缓存更新: {} -> {}",
                packet.sender_ip, packet.sender_mac
            );
        }
        self.print_cache();
        println!("---------------End of ARP Protocol---------------");
        Ok(())
    }

    fn print_cache(&self) {
        if self.cache.is_empty() {
            println!("ARP 缓存：<空>");
            return;
        }
        println!("ARP 缓存：");
        let mut entries: Vec<_> = self.cache.iter().collect();
        entries.sort_by_key(|(ip, _)| **ip);
        for (ip, mac) in entries {
            println!("  {ip} -> {mac}");
        }
    }
}
