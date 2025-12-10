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

use anyhow::{Result, bail};

use crate::ipv4_addr::Ipv4Addr;
use crate::mac::MacAddr;

const ARP_FIXED_LEN: usize = 28;
const HW_TYPE_ETHERNET: u16 = 0x0001;
const PROTO_TYPE_IPV4: u16 = 0x0800;

#[derive(Debug, Clone, Copy)]
pub struct ArpPacket {
    pub hardware_type: u16,
    pub protocol_type: u16,
    pub hardware_len: u8,
    pub protocol_len: u8,
    pub opcode: u16,
    pub sender_mac: MacAddr,
    pub sender_ip: Ipv4Addr,
    pub target_mac: MacAddr,
    pub target_ip: Ipv4Addr,
}

impl ArpPacket {
    pub fn parse(payload: &[u8]) -> Result<Self> {
        if payload.len() < ARP_FIXED_LEN {
            bail!("ARP 负载长度 {} 不足 {ARP_FIXED_LEN}", payload.len());
        }
        let hardware_type = u16::from_be_bytes([payload[0], payload[1]]);
        let protocol_type = u16::from_be_bytes([payload[2], payload[3]]);
        let hardware_len = payload[4];
        let protocol_len = payload[5];
        let opcode = u16::from_be_bytes([payload[6], payload[7]]);

        if hardware_len as usize != 6 || protocol_len as usize != 4 {
            bail!(
                "暂不支持硬件长度 {} / 协议长度 {} 的 ARP 报文",
                hardware_len,
                protocol_len
            );
        }

        let sender_mac = MacAddr::from_slice(&payload[8..14]);
        let sender_ip = Ipv4Addr::new(payload[14], payload[15], payload[16], payload[17]);
        let target_mac = MacAddr::from_slice(&payload[18..24]);
        let target_ip = Ipv4Addr::new(payload[24], payload[25], payload[26], payload[27]);

        Ok(Self {
            hardware_type,
            protocol_type,
            hardware_len,
            protocol_len,
            opcode,
            sender_mac,
            sender_ip,
            target_mac,
            target_ip,
        })
    }

    pub fn is_ethernet_ipv4(&self) -> bool {
        self.hardware_type == HW_TYPE_ETHERNET && self.protocol_type == PROTO_TYPE_IPV4
    }

    pub fn opcode_label(&self) -> &'static str {
        match self.opcode {
            1 => "Request",
            2 => "Reply",
            _ => "未知",
        }
    }
}

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

    pub fn process(&mut self, payload: &[u8]) -> Result<()> {
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
