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

use std::{fs, thread, time::Duration};

use anyhow::{Context, Result, bail};
use pcap::{Active, Capture, Device};
use protocol::checksum::Crc32;

use crate::arp::build_arp_payload;
use crate::cli::{ArpMode, SendArgs};
use crate::ipv4::{Ipv4Config, build_ipv4_payloads};
use protocol::arp::ArpOperation;

const MIN_PAYLOAD: usize = 46;
const MAX_PAYLOAD: usize = 1500;
const HEADER_LEN: usize = 14;
const CRC_LEN: usize = 4;

pub fn list_adapters() -> Result<()> {
    let devices = Device::list().context("枚举网卡接口失败")?;
    if devices.is_empty() {
        println!("libpcap 未报告任何可用接口。");
        return Ok(());
    }
    for (idx, dev) in devices.iter().enumerate() {
        match dev.desc.as_deref() {
            Some(desc) => println!("{}. {} ({})", idx + 1, dev.name, desc),
            None => println!("{}. {} (无描述)", idx + 1, dev.name),
        }
    }
    Ok(())
}

pub fn send_packets(args: SendArgs) -> Result<()> {
    let payloads = if let Some(arp_mode) = args.arp_mode {
        let mut payload = build_arp_payload(
            map_arp_mode(arp_mode),
            args.src_mac,
            args.src_ip,
            args.arp_target_mac,
            args.arp_target_ip,
        );
        enforce_payload_rules(&mut payload, true)?;
        vec![payload]
    } else {
        let mut file_payload =
            fs::read(&args.data).with_context(|| format!("无法从 {:?} 读取载荷", args.data))?;

        if args.ipv4 {
            let cfg = Ipv4Config {
                src_ip: args.src_ip,
                dst_ip: args.dst_ip,
                ttl: args.ttl,
                tos: args.tos,
                protocol: args.protocol,
                fragment_size: args.fragment_size,
                identification: args.ip_id,
                dont_fragment: args.dont_fragment,
            };
            build_ipv4_payloads(&file_payload, &cfg)?
        } else {
            enforce_payload_rules(&mut file_payload, args.pad)?;
            vec![file_payload]
        }
    };

    if payloads.is_empty() {
        bail!("未生成任何以太网载荷，请检查输入文件。");
    }

    let ethertype = if args.arp_mode.is_some() {
        0x0806
    } else if args.ipv4 {
        0x0800
    } else {
        args.ethertype
    };
    let crc = Crc32::new();
    let frames: Vec<Vec<u8>> = payloads
        .into_iter()
        .map(|payload| assemble_frame(&args, payload, ethertype, &crc))
        .collect();

    let mut capture = open_interface(&args.interface, args.timeout_ms)?;

    let mut sent = 0u64;
    let mut frame_index = 0usize;
    loop {
        let frame = &frames[frame_index];
        capture
            .sendpacket(frame.as_slice())
            .with_context(|| "pcap 无法发送构造的帧")?;
        sent += 1;

        if args.count.is_some_and(|max| sent >= max) {
            break;
        }

        frame_index = (frame_index + 1) % frames.len();

        if args.interval_ms > 0 {
            thread::sleep(Duration::from_millis(args.interval_ms));
        }
    }

    println!(
        "已发送 {sent} 个帧，每帧总长度 {} 字节（帧头 {} + CRC {} + 动态载荷）。",
        frames
            .first()
            .map(|f| f.len())
            .unwrap_or(HEADER_LEN + CRC_LEN),
        HEADER_LEN,
        CRC_LEN
    );
    Ok(())
}

fn assemble_frame(args: &SendArgs, payload: Vec<u8>, ethertype: u16, crc: &Crc32) -> Vec<u8> {
    let checksum = crc.checksum(&payload);
    let mut frame = Vec::with_capacity(HEADER_LEN + payload.len() + CRC_LEN);
    frame.extend_from_slice(args.dest_mac.as_bytes());
    frame.extend_from_slice(args.src_mac.as_bytes());
    frame.extend_from_slice(&ethertype.to_be_bytes());
    frame.extend_from_slice(&payload);
    frame.extend_from_slice(&checksum.to_le_bytes());
    frame
}

fn open_interface(name: &str, timeout_ms: i32) -> Result<Capture<Active>> {
    Capture::from_device(name)
        .with_context(|| format!("找不到接口 {name}"))?
        .promisc(true)
        .snaplen(65535)
        .timeout(timeout_ms)
        .open()
        .with_context(|| format!("打开接口 {name} 失败"))
}

fn enforce_payload_rules(payload: &mut Vec<u8>, pad: bool) -> Result<()> {
    if payload.len() > MAX_PAYLOAD {
        bail!(
            "载荷大小 {} 超出以太网最大 {} 字节限制",
            payload.len(),
            MAX_PAYLOAD
        );
    }

    if payload.len() < MIN_PAYLOAD {
        if pad {
            payload.resize(MIN_PAYLOAD, 0);
        } else {
            bail!(
                "载荷大小 {} 小于以太网最小 {} 字节（使用 --pad 自动补零）",
                payload.len(),
                MIN_PAYLOAD
            );
        }
    }
    Ok(())
}

fn map_arp_mode(mode: ArpMode) -> ArpOperation {
    match mode {
        ArpMode::Request => ArpOperation::Request,
        ArpMode::Reply => ArpOperation::Reply,
    }
}
