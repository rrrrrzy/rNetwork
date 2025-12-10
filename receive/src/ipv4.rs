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

use std::{
    collections::HashMap,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
    time::{Duration, Instant},
};

use crate::checksum::simple_checksum as ipv4_checksum;
use crate::ipv4_addr::Ipv4Addr;

use anyhow::Result;

pub struct Ipv4Processor {
    accepted: Vec<Ipv4Addr>,
    writer: BufWriter<File>,
    buffers: HashMap<FragmentKey, FragmentBuffer>,
    fragment_timeout: Duration,
}

impl Ipv4Processor {
    pub fn new(accepted: Vec<Ipv4Addr>, output: &PathBuf) -> Result<Self> {
        let mut allow = accepted;
        if allow.is_empty() {
            allow.push(Ipv4Addr::broadcast());
            allow.push(Ipv4Addr::new(192, 168, 0, 1));
        }
        allow.sort();
        allow.dedup();
        let writer = BufWriter::new(File::create(output)?);
        Ok(Self {
            accepted: allow,
            writer,
            buffers: HashMap::new(),
            fragment_timeout: Duration::from_secs(30),
        })
    }

    pub fn allowed_destinations(&self) -> String {
        self.accepted
            .iter()
            .map(|ip| ip.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn allowed_slice(&self) -> &[Ipv4Addr] {
        &self.accepted
    }

    pub fn process(&mut self, payload: &[u8]) -> Result<()> {
        if payload.len() < 20 {
            println!("IPv4 数据长度不足，丢弃。");
            return Ok(());
        }

        let version = payload[0] >> 4;
        let header_len = ((payload[0] & 0x0F) as usize) * 4;
        if version != 4 || header_len < 20 || payload.len() < header_len {
            println!("IPv4 头部格式非法，丢弃。");
            return Ok(());
        }

        let total_length = u16::from_be_bytes([payload[2], payload[3]]) as usize;
        if total_length > payload.len() {
            println!("IPv4 长度字段超出实际数据，丢弃。");
            return Ok(());
        }

        let ttl = payload[8];
        if ttl == 0 {
            println!("TTL 已耗尽，丢弃该 IP 数据包。");
            return Ok(());
        }

        let header_checksum = u16::from_be_bytes([payload[10], payload[11]]);
        let computed = ipv4_checksum(&payload[..header_len]);
        if computed != 0 {
            println!("IPv4 头部校验和错误 (期望 0，实际 {computed:#06x})。");
            return Ok(());
        }

        let src = Ipv4Addr::new(payload[12], payload[13], payload[14], payload[15]);
        let dst = Ipv4Addr::new(payload[16], payload[17], payload[18], payload[19]);

        if !self.accepted.is_empty() && !self.accepted.contains(&dst) {
            println!("目的 IP {dst} 不在白名单，忽略。");
            return Ok(());
        }

        println!("--------------IP Protocol-------------------");
        println!("IP 版本: {version}");
        println!("头部长度: {header_len} 字节");
        println!("服务类型: 0x{:02x}", payload[1]);
        println!("报文总长: {}", total_length);
        let identification = u16::from_be_bytes([payload[4], payload[5]]);
        println!("标识符: {identification}");
        let fragment_field = u16::from_be_bytes([payload[6], payload[7]]);
        let more_fragments = fragment_field & 0x2000 != 0;
        let fragment_offset = (fragment_field & 0x1FFF) * 8;
        println!(
            "分片标志/偏移: 0x{fragment_field:04x} (偏移 {} 字节, MF={})",
            fragment_offset,
            if more_fragments { "是" } else { "否" }
        );
        println!("TTL: {ttl}");
        println!("上层协议: 0x{:02x}", payload[9]);
        println!("头部校验和: 0x{header_checksum:04x}");
        println!("源 IP: {src}");
        println!("目的 IP: {dst}");

        let data_end = total_length.min(payload.len());
        let ip_data = &payload[header_len..data_end];

        if fragment_offset == 0 && !more_fragments {
            self.writer.write_all(ip_data)?;
            self.writer.flush()?;
            println!("未分片数据已写入 IPv4 输出文件。");
            print_ipv4_payload("IPv4 数据内容", ip_data);
        } else {
            self.handle_fragment(
                src,
                dst,
                identification,
                fragment_offset as usize,
                more_fragments,
                ip_data,
            )?;
        }

        println!("-----------------End of IP Protocol---------------");
        Ok(())
    }

    fn handle_fragment(
        &mut self,
        src: Ipv4Addr,
        dst: Ipv4Addr,
        id: u16,
        offset: usize,
        more: bool,
        data: &[u8],
    ) -> Result<()> {
        let now = Instant::now();
        self.buffers
            .retain(|_, entry| now.duration_since(entry.last_update) <= self.fragment_timeout);

        let key = FragmentKey { src, dst, id };
        let entry = self.buffers.entry(key).or_insert_with(|| FragmentBuffer {
            data: Vec::new(),
            last_update: now,
            more_expected: true,
            next_offset: 0,
        });

        if offset != entry.next_offset {
            println!(
                "分片偏移与已接收数据不连续 (期待 {} 字节，实际 {} 字节)，丢弃该报文。",
                entry.next_offset, offset
            );
            self.buffers.remove(&key);
            return Ok(());
        }

        entry.data.extend_from_slice(data);
        entry.next_offset += data.len();
        entry.more_expected = more;
        entry.last_update = now;

        if !more {
            self.writer.write_all(&entry.data)?;
            self.writer.flush()?;
            println!(
                "分片报文 (ID={id}) 重组完成，总长度 {} 字节，已写入文件。",
                entry.data.len()
            );
            print_ipv4_payload("IPv4 分片重组数据", &entry.data);
            self.buffers.remove(&key);
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
struct FragmentKey {
    src: Ipv4Addr,
    dst: Ipv4Addr,
    id: u16,
}

#[derive(Debug)]
struct FragmentBuffer {
    data: Vec<u8>,
    last_update: Instant,
    more_expected: bool,
    next_offset: usize,
}

fn print_ipv4_payload(label: &str, data: &[u8]) {
    if data.is_empty() {
        println!("{label}: <空>");
        return;
    }
    match std::str::from_utf8(data) {
        Ok(text) => println!("{label} (UTF-8): {text}"),
        Err(_) => {
            print!("{label}: ");
            for &byte in data {
                if byte.is_ascii_graphic() || byte == b' ' {
                    print!("{}", byte as char);
                } else if byte == b'\n' {
                    print!("\\n");
                } else if byte == b'\r' {
                    print!("\\r");
                } else {
                    print!("\\x{:02X}", byte);
                }
            }
            println!();
            println!("(UTF-8 解析失败，已回退到逐字节转储)");
        }
    }
}
