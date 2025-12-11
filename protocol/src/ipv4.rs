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

use crate::checksum::simple_checksum;
use crate::error::Ipv4HeaderParseError;
use crate::error::Ipv4ParseError;
use std::fmt;
use std::str::FromStr;

/// 自定义 IPv4 地址类型，减少对标准库的依赖
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Ipv4Addr([u8; 4]);

impl Ipv4Addr {
    /// 从四个字节构造 IPv4 地址
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    /// 从字节数组构造
    pub const fn from_octets(octets: [u8; 4]) -> Self {
        Self(octets)
    }

    /// 获取字节数组
    pub const fn octets(&self) -> [u8; 4] {
        self.0
    }

    /// 广播地址 255.255.255.255
    pub const fn broadcast() -> Self {
        Self([255, 255, 255, 255])
    }

    /// 未指定地址 0.0.0.0
    pub const fn unspecified() -> Self {
        Self([0, 0, 0, 0])
    }

    /// 本地回环地址 127.0.0.1
    pub const fn localhost() -> Self {
        Self([127, 0, 0, 1])
    }
}

impl fmt::Display for Ipv4Addr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

impl FromStr for Ipv4Addr {
    type Err = Ipv4ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 4 {
            return Err(Ipv4ParseError::InvalidFormat);
        }

        let mut octets = [0u8; 4];
        for (i, part) in parts.iter().enumerate() {
            octets[i] = part.parse().map_err(|_| Ipv4ParseError::InvalidOctet)?;
        }

        Ok(Self(octets))
    }
}

impl fmt::Display for Ipv4ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "IPv4 address format error, should be: a.b.c.d"),
            Self::InvalidOctet => write!(f, "IPv4 address num range error, should be in 0-255"),
        }
    }
}

impl std::error::Error for Ipv4ParseError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv4Header {
    pub version: u8,      // 4 bits
    pub ihl: u8,          // 4 bits (Internet Header Length)
    pub tos: u8,          // Type of Service
    pub total_len: u16,   // Total length
    pub id: u16,          // Identification
    pub flags: u8,        // 3 bits (R, DF, MF)
    pub frag_offset: u16, // 13 bits
    pub ttl: u8,          // Time to live
    pub protocol: u8,     // Protocol (TCP = 6, UDP = 17, ICMP = 1)
    pub checksum: u16,    // Header checksum
    pub src: Ipv4Addr,    // Source address
    pub dst: Ipv4Addr,    // Destination address
                          // pub option: [u8; 40], // Option + fill, not support now!
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ipv4Protocol {
    ICMP,
    TCP,
    UDP,
    Unknown,
}

impl Ipv4Header {
    pub fn new(src: Ipv4Addr, dst: Ipv4Addr, protocol: u8, payload_len: u16, id: u16) -> Self {
        // notice that we do not support option and fill currently
        let total_len = payload_len + 20;
        let mut header = Self {
            version: 4,      // IPv4
            ihl: 5,          // 1 = 4 Bytes
            tos: 0b00011110, // priority = 0, D = 1, T = 1, R = 1, C = 1, 0(no meaning)
            total_len,       // set by user
            id,              // Identificaion, set from 0 to max value by controller
            flags: 0,        // 0(no meaning), DF = 0, MF = 0. we do not support it currently
            frag_offset: 0,  // DF = 0 ==> frag_offset = 0
            ttl: 64,         // usually be 64
            protocol,        // set by user
            checksum: 0,     // temporarily set to 0
            src,             // set by user
            dst,             // set by user
                             // option: [0; 40], // empty, we do not support it currently
        };
        header.checksum = header.checksum();

        header
    }

    #[allow(clippy::clone_on_copy)] // 这里使用clone来显示声明拷贝了一份，用*self太Rust了，容易引起误解
    pub fn checksum(&self) -> u16 {
        if self.checksum != 0 {
            let mut ipv4_header = self.clone();
            ipv4_header.checksum = 0;
            simple_checksum(&ipv4_header.to_bytes())
        } else {
            simple_checksum(&self.to_bytes())
        }
    }

    pub fn get_protocol(&self) -> Ipv4Protocol {
        match self.protocol {
            1 => Ipv4Protocol::ICMP,
            6 => Ipv4Protocol::TCP,
            17 => Ipv4Protocol::UDP,
            _ => Ipv4Protocol::Unknown,
        }
    }

    pub fn to_bytes(&self) -> [u8; 20] {
        let mut bytes: [u8; 20] = [0; 20];
        bytes[0] = (self.version << 4) + self.ihl;
        bytes[1] = self.tos;
        bytes[2..4].copy_from_slice(&self.total_len.to_be_bytes());
        bytes[4..6].copy_from_slice(&self.id.to_be_bytes());
        bytes[6] = (self.flags << 5) + (self.frag_offset.to_be_bytes()[0] & 0b00011111);
        bytes[7] = self.frag_offset.to_be_bytes()[1];
        bytes[8] = self.ttl;
        bytes[9] = self.protocol;
        bytes[10..12].copy_from_slice(&self.checksum.to_be_bytes());
        bytes[12..16].copy_from_slice(&self.src.0);
        bytes[16..20].copy_from_slice(&self.dst.0);

        bytes
    }

    pub fn validate(&self) -> Result<(), Ipv4HeaderParseError> {
        if self.checksum != self.checksum() {
            Err(Ipv4HeaderParseError::InvalidChecksum)
        } else if self.version != 4 {
            Err(Ipv4HeaderParseError::InvalidVersion)
        } else if self.ihl < 5 || self.ihl > 15 {
            // IHL 单位是 4字节，所以 5 代表 20字节，15 代表 60字节
            Err(Ipv4HeaderParseError::InvalidHeaderLength)
        } else if self.ttl == 0 {
            Err(Ipv4HeaderParseError::InvalidTimeToLive)
        } else {
            Ok(())
        }
    }

    pub fn parse(bytes: &[u8]) -> Result<Self, Ipv4HeaderParseError> {
        let version = bytes[0] >> 4;
        let ihl = bytes[0] & 0x0F;
        let tos = bytes[1];
        let total_len = u16::from_be_bytes(
            bytes[2..4]
                .try_into()
                .map_err(|_| Ipv4HeaderParseError::InvalidHeaderLength)?,
        );
        let id = u16::from_be_bytes(
            bytes[4..6]
                .try_into()
                .map_err(|_| Ipv4HeaderParseError::InvalidHeaderLength)?,
        );
        let flags = bytes[6] >> 5;
        let frag_offset = u16::from_be_bytes([bytes[6] & 0b00011111, bytes[7]]);
        let ttl = bytes[8];
        let protocol = bytes[9];
        let checksum = u16::from_be_bytes(
            bytes[10..12]
                .try_into()
                .map_err(|_| Ipv4HeaderParseError::InvalidHeaderLength)?,
        );
        let src = Ipv4Addr::from_octets(
            bytes[12..16]
                .try_into()
                .map_err(|_| Ipv4HeaderParseError::InvalidHeaderLength)?,
        );
        let dst = Ipv4Addr::from_octets(
            bytes[16..20]
                .try_into()
                .map_err(|_| Ipv4HeaderParseError::InvalidHeaderLength)?,
        );

        let ipv4_header = Self {
            version,
            ihl,
            tos,
            total_len,
            id,
            flags,
            frag_offset,
            ttl,
            protocol,
            checksum,
            src,
            dst,
        };

        ipv4_header.validate()?;
        Ok(ipv4_header)
    }
}

impl fmt::Display for Ipv4Header {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IPv4 Header:
    Version: {}
    IHL: {} ({} bytes)
    TOS: {}
    Total Length: {}
    Identification: {}
    Flags: {:03b} (DF={}, MF={})
    Fragment Offset: {}
    TTL: {}
    Protocol: {} ({:?})
    Checksum: {:#06x}
    Source: {}
    Destination: {}",
            self.version,
            self.ihl,
            self.ihl * 4,
            self.tos,
            self.total_len,
            self.id,
            self.flags,
            (self.flags >> 1) & 1,
            self.flags & 1,
            self.frag_offset,
            self.ttl,
            self.protocol,
            self.get_protocol(),
            self.checksum,
            self.src,
            self.dst
        )
    }
}
