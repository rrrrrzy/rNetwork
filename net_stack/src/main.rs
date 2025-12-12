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

use anyhow::{Context, Result};
use clap::Parser;
use pcap::{Capture, Device};
use protocol::{ipv4::Ipv4Addr, mac::MacAddr};
use std::fs;
use std::str::FromStr;

mod cli;
mod handlers;
mod stack;
mod transport;

use cli::Args;
use handlers::icmp;
use stack::{NetworkStack, StackConfig};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::transport::SocketSet;

fn main() -> Result<()> {
    let args = Args::parse();

    // 从配置文件或命令行参数获取 IP 和 MAC
    let (ip_str, mac_str) = if let Some(config_path) = &args.config {
        load_config_from_file(config_path)?
    } else {
        let ip = args
            .ip
            .ok_or_else(|| anyhow::anyhow!("--ip is required when not using --config"))?;
        let mac = args
            .mac
            .ok_or_else(|| anyhow::anyhow!("--mac is required when not using --config"))?;
        (ip, mac)
    };

    let ip = Ipv4Addr::from_str(&ip_str).map_err(|e| anyhow::anyhow!("{}", e))?;
    let mac = MacAddr::from_str(&mac_str).map_err(|e| anyhow::anyhow!("{}", e))?;

    let config = StackConfig { mac, ip };
    let socket_set = SocketSet::new();

    // 1. Find the device
    let device = Device::list()?
        .into_iter()
        .find(|d| d.name == args.iface)
        .ok_or_else(|| anyhow::anyhow!("Device not found: {}", args.iface))?;

    println!("Starting Network Stack on interface: {}", device.name);

    // 2. Open Capture for RX (Receive)
    let mut rx_cap = Capture::from_device(device.clone())?
        .promisc(true)
        .snaplen(65535)
        .timeout(10) // 10ms timeout for non-blocking feel
        .open()?;

    // 3. Open Capture for TX (Send)
    let tx_cap = Capture::from_device(device)?
        .promisc(true)
        .snaplen(65535)
        .open()?;

    // 4. Initialize Stack
    let stack = Arc::new(NetworkStack::new(config, tx_cap, socket_set));

    println!("Stack initialized. IP: {}, MAC: {}", ip, mac);

    // 5. Start Ping Thread if requested
    if let Some(target_ip_str) = args.ping {
        let target_ip = Ipv4Addr::from_str(&target_ip_str)
            .map_err(|e| anyhow::anyhow!("Invalid target IP: {}", e))?;

        let target_mac = if let Some(mac_str) = args.target_mac {
            MacAddr::from_str(&mac_str).map_err(|e| anyhow::anyhow!("Invalid target MAC: {}", e))?
        } else {
            println!(
                "Warning: No target MAC specified, using Broadcast. This may not work for routed traffic."
            );
            MacAddr::broadcast()
        };

        println!("Starting Ping to {} (MAC: {})", target_ip, target_mac);

        let stack_clone = stack.clone();
        thread::spawn(move || {
            let mut seq = 1;
            loop {
                println!("Sending ICMP Request seq={} to {}", seq, target_ip);
                icmp::send_icmp_request(&stack_clone, target_mac, target_ip, seq);
                seq += 1;
                thread::sleep(Duration::from_secs(1));
            }
        });
    }

    println!("Waiting for packets...");

    // 6. Event Loop
    loop {
        match rx_cap.next_packet() {
            Ok(packet) => {
                stack.receive(packet.data);
            }
            Err(pcap::Error::TimeoutExpired) => {
                continue;
            }
            Err(e) => {
                eprintln!("Error receiving packet: {:?}", e);
            }
        }
    }
}

fn load_config_from_file(path: &std::path::Path) -> Result<(String, String)> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;

    let mut ip = None;
    let mut mac = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "ip" => ip = Some(value.to_string()),
                "mac" => mac = Some(value.to_string()),
                _ => eprintln!("Warning: Unknown config key: {}", key),
            }
        }
    }

    let ip = ip.ok_or_else(|| anyhow::anyhow!("Missing 'ip' in config file"))?;
    let mac = mac.ok_or_else(|| anyhow::anyhow!("Missing 'mac' in config file"))?;

    Ok((ip, mac))
}
