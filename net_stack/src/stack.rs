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

use pcap::{Active, Capture};
use protocol::ethernet::{EtherType, EthernetHeader};
use protocol::ipv4::{Ipv4Addr, Ipv4Header, Ipv4Protocol};
use protocol::mac::MacAddr;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// 引入 handlers
use crate::handlers::{arp, ipv4};
use crate::transport::SocketSet;
use protocol::arp::ArpTable;

pub struct PendingPacket {
    pub dst_ip: Ipv4Addr,
    pub protocol: Ipv4Protocol,
    pub payload: Vec<u8>,
    pub timestamp: Instant,
}

pub struct StackConfig {
    pub mac: MacAddr,
    pub ip: Ipv4Addr,
    // pub gateway: Ipv4Addr, // 以后再加
}

pub struct NetworkStack {
    config: StackConfig,
    // 发送端需要互斥锁，因为可能有多个线程（RX线程回包，用户线程发包）同时发送
    sender: Arc<Mutex<Capture<Active>>>,
    arp_table: Arc<Mutex<ArpTable>>,
    pub sockets: Arc<Mutex<SocketSet>>,
    pending_packets: Arc<Mutex<HashMap<Ipv4Addr, VecDeque<PendingPacket>>>>,
}

impl NetworkStack {
    pub fn new(config: StackConfig, sender: Capture<Active>, socket: SocketSet) -> Self {
        Self {
            config,
            sender: Arc::new(Mutex::new(sender)),
            arp_table: Arc::new(Mutex::new(ArpTable::new(Duration::from_secs(300)))),
            sockets: Arc::new(Mutex::new(socket)),
            pending_packets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // 核心入口：处理收到的以太网帧
    pub fn receive(&self, packet: &[u8]) {
        // 1. 解析以太网头
        let eth_header = match EthernetHeader::parse(packet) {
            Ok(h) => h,
            Err(e) => {
                eprintln!("Invalid Ethernet frame: {}", e);
                return;
            }
        };

        // 2. 过滤：只处理发给我的，或者广播包
        if eth_header.dst != self.config.mac && eth_header.dst != MacAddr::broadcast() {
            return;
        }

        // 3. 剥离以太网头，获取 Payload
        let payload = &packet[EthernetHeader::LEN..];

        // 4. 分发
        match eth_header.ethertype {
            EtherType::Arp => {
                // 调用 ARP Handler
                arp::handle(self, payload);
            }
            EtherType::Ipv4 => {
                // 调用 IPv4 Handler
                ipv4::handle(self, payload);
            }
            EtherType::Ipv6 => {
                // println!("IPv6 is not supported");
            }
            _ => {
                println!("Unknown EtherType: {}", eth_header.ethertype);
            }
        }
    }

    // 发送接口：发送以太网帧
    pub fn send_frame(&self, frame: &[u8]) {
        if let Ok(mut sender) = self.sender.lock() {
            let _ = sender.sendpacket(frame);
        }
    }

    // 辅助接口：获取本机配置
    pub fn config(&self) -> &StackConfig {
        &self.config
    }

    // 获取 ARP 表
    pub fn arp_table(&self) -> &Arc<Mutex<ArpTable>> {
        &self.arp_table
    }

    // 获取待发送的 IP 包列表
    pub fn pending_packets(&self) -> &Arc<Mutex<HashMap<Ipv4Addr, VecDeque<PendingPacket>>>> {
        &self.pending_packets
    }
}
