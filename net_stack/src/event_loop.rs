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
use anyhow::Result;
use protocol::ipv4::Ipv4Addr;
use std::str::FromStr;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

pub fn run(stack: Arc<NetworkStack>) -> Result<()> {
    println!("Entering event loop...");

    // 这里唯一占有该锁
    let mut rx_cap = stack.get_rx_capture().lock().unwrap();
    let mut last_cleanup = Instant::now();

    loop {
        // 1. 接收
        match rx_cap.next_packet() {
            Ok(packet) => stack.receive(packet.data),
            Err(pcap::Error::TimeoutExpired) => {}
            Err(e) => eprintln!("RX Error: {:?}", e),
        }

        // 2. 发送
        stack.poll_and_send();

        // 3. 清理
        if last_cleanup.elapsed() > Duration::from_secs(1) {
            stack.cleanup_pending_packets();
            last_cleanup = Instant::now();
        }
    }
}

pub fn ping(target_ip_str: &str, stack: &Arc<NetworkStack>) -> Result<()> {
    let target_ip = Ipv4Addr::from_str(target_ip_str)
        .map_err(|e| anyhow::anyhow!("Invalid target IP: {}", e))?;

    println!("Starting Ping to {}", target_ip);

    let stack_clone = stack.clone();
    thread::spawn(move || {
        let mut seq = 1;
        loop {
            println!("Sending ICMP Request seq={} to {}", seq, target_ip);
            handlers::icmp::send_icmp_request(&stack_clone, target_ip, seq);
            seq += 1;
            thread::sleep(Duration::from_secs(1));
        }
    });
    Ok(())
}
