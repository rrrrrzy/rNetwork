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

use crate::error::EthernetParseError;
use crate::mac::MacAddr;
use std::fmt;

#[repr(C, u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EtherType {
    Ipv4 = 0x0800,
    Arp = 0x0806,
    Ipv6 = 0x86DD,
    Unknown(u16),
}

impl From<u16> for EtherType {
    fn from(val: u16) -> Self {
        match val {
            0x0800 => EtherType::Ipv4,
            0x0806 => EtherType::Arp,
            0x86DD => EtherType::Ipv6,
            _ => EtherType::Unknown(val),
        }
    }
}

impl From<EtherType> for u16 {
    fn from(val: EtherType) -> Self {
        match val {
            EtherType::Ipv4 => 0x0800,
            EtherType::Arp => 0x0806,
            EtherType::Ipv6 => 0x86DD,
            EtherType::Unknown(v) => v,
        }
    }
}

impl fmt::Display for EtherType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EtherType::Ipv4 => write!(f, "IPv4 (0x0800)"),
            EtherType::Arp => write!(f, "ARP (0x0806)"),
            EtherType::Ipv6 => write!(f, "IPv6 (0x86DD)"),
            EtherType::Unknown(v) => write!(f, "Unknown (0x{:04X})", v),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EthernetHeader {
    pub dst: MacAddr,
    pub src: MacAddr,
    pub ethertype: EtherType,
}

impl fmt::Display for EthernetParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EthernetParseError::PacketTooShort => {
                write!(f, "Packet is too short to contain an Ethernet header")
            }
        }
    }
}

impl std::error::Error for EthernetParseError {}

impl EthernetHeader {
    pub const LEN: usize = 14;

    pub fn new(src: MacAddr, dst: MacAddr, ethertype: EtherType) -> Self {
        Self {
            src,
            dst,
            ethertype,
        }
    }

    pub fn parse(data: &[u8]) -> Result<Self, EthernetParseError> {
        if data.len() < Self::LEN {
            return Err(EthernetParseError::PacketTooShort);
        }

        let dst = MacAddr::from_slice(&data[0..6]);
        let src = MacAddr::from_slice(&data[6..12]);
        let ethertype_val = u16::from_be_bytes([data[12], data[13]]);

        Ok(Self {
            dst,
            src,
            ethertype: EtherType::from(ethertype_val),
        })
    }

    pub fn to_bytes(&self) -> [u8; 14] {
        let mut bytes = [0u8; 14];
        bytes[0..6].copy_from_slice(self.dst.as_bytes());
        bytes[6..12].copy_from_slice(self.src.as_bytes());
        let ethertype_val: u16 = self.ethertype.into();
        bytes[12..14].copy_from_slice(&ethertype_val.to_be_bytes());
        bytes
    }
}

impl fmt::Display for EthernetHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Ethernet Header:
    Source: {}
    Destination: {}
    EtherType: {}",
            self.src, self.dst, self.ethertype
        )
    }
}
