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

use crate::error::ArpParseError;
use crate::ipv4::Ipv4Addr;
use crate::mac::MacAddr;

/// arp parameters
const ARP_FIXED_LEN: usize = 28;
const HW_TYPE_ETHERNET: u16 = 0x0001;
const PROTO_TYPE_IPV4: u16 = 0x0800;

/// Supported ARP operations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArpOperation {
    Request,
    Reply,
    Unknown,
}

impl ArpOperation {
    pub fn opcode(self) -> u16 {
        match self {
            Self::Request => 1,
            Self::Reply => 2,
            Self::Unknown => 0,
        }
    }

    pub fn parse(opcode: u16) -> Self {
        match opcode {
            1 => Self::Request,
            2 => Self::Reply,
            _ => Self::Unknown,
        }
    }
}

impl std::fmt::Display for ArpOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request => write!(f, "Request"),
            Self::Reply => write!(f, "Reply"),
            Self::Unknown => write!(f, "Unknown"),
        }
    }
}

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
    pub fn new(
        op: ArpOperation,
        sender_mac: MacAddr,
        sender_ip: Ipv4Addr,
        target_mac: MacAddr,
        target_ip: Ipv4Addr,
    ) -> Self {
        Self {
            hardware_type: 1,      // the project use Ethernet protocol
            protocol_type: 0x0800, // the project use IP protocol
            hardware_len: 6,       // len of MAC
            protocol_len: 4,       // len of IPv4
            opcode: op.opcode(),
            sender_mac,
            sender_ip,
            target_mac,
            target_ip,
        }
    }

    pub fn parse(payload: &[u8]) -> Result<Self, ArpParseError> {
        if payload.len() < ARP_FIXED_LEN {
            eprintln!("ARP 负载长度 {} 不足 {ARP_FIXED_LEN}", payload.len());
            return Err(ArpParseError::InvalidArpFixedLength);
        }
        let hardware_type = u16::from_be_bytes([payload[0], payload[1]]);
        let protocol_type = u16::from_be_bytes([payload[2], payload[3]]);
        let hardware_len = payload[4];
        let protocol_len = payload[5];
        let opcode = u16::from_be_bytes([payload[6], payload[7]]);

        if hardware_len as usize != 6 {
            eprintln!("暂不支持硬件长度 {} 的 ARP 报文", hardware_len,);
            return Err(ArpParseError::InvalidArpHardwareLength);
        } else if protocol_len as usize != 4 {
            eprintln!("暂不支持协议长度 {} 的 ARP 报文", protocol_len);
            return Err(ArpParseError::InvalidArpProtocolLength);
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

    pub fn opcode_label(&self) -> ArpOperation {
        match self.opcode {
            1 => ArpOperation::Request,
            2 => ArpOperation::Reply,
            _ => ArpOperation::Unknown,
        }
    }

    pub fn to_bytes(&self) -> [u8; 28] {
        let mut bytes = [0u8; 28];
        bytes[0..2].copy_from_slice(&self.hardware_type.to_be_bytes());
        bytes[2..4].copy_from_slice(&self.protocol_type.to_be_bytes());
        bytes[4] = self.hardware_len;
        bytes[5] = self.protocol_len;
        bytes[6..8].copy_from_slice(&self.opcode.to_be_bytes());
        bytes[8..14].copy_from_slice(self.sender_mac.as_bytes());
        bytes[14..18].copy_from_slice(&self.sender_ip.octets());
        bytes[18..24].copy_from_slice(self.target_mac.as_bytes());
        bytes[24..28].copy_from_slice(&self.target_ip.octets());

        bytes
    }
}
